/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#include <cstdint>
#include <cstdio>
#include <array>
#include <optional>
#include <memory>

#include "dynarmic/interface/A64/a64.h"
#include "dynarmic/interface/A64/config.h"
#include "dynarmic/interface/exclusive_monitor.h"

namespace touchHLE::cpu {

using VAddr = std::uint64_t;
using Vector = Dynarmic::A64::Vector;

// Types and functions defined in Rust
extern "C" {
struct touchHLE_Mem;
std::uint8_t touchHLE_cpu_read_u8(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint16_t touchHLE_cpu_read_u16(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint32_t touchHLE_cpu_read_u32(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint64_t touchHLE_cpu_read_u64(touchHLE_Mem *mem, VAddr addr, bool *error);
bool touchHLE_cpu_write_u8(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
bool touchHLE_cpu_write_u16(touchHLE_Mem *mem, VAddr addr, std::uint16_t value);
bool touchHLE_cpu_write_u32(touchHLE_Mem *mem, VAddr addr, std::uint32_t value);
bool touchHLE_cpu_write_u64(touchHLE_Mem *mem, VAddr addr, std::uint64_t value);

// TODO: hay que rediseñar este contexto para A64
// (regs de 64 bits + vectores)
struct touchHLE_DynarmicContext64 {
  std::array<std::uint64_t, 31> regs;   // X0-X30
  std::uint64_t sp;
  std::uint64_t pc;
  std::uint32_t pstate;
  // TODO: FP/SIMD registers
};
}

const auto HaltReasonSvc = Dynarmic::HaltReason::UserDefined1;
const auto HaltReasonUndefinedInstruction = Dynarmic::HaltReason::UserDefined2;
const auto HaltReasonBreakpoint = Dynarmic::HaltReason::UserDefined3;

class Environment64 final : public Dynarmic::A64::UserCallbacks {
public:
  Dynarmic::A64::Jit *cpu = nullptr;
  touchHLE_Mem *mem = nullptr;
  std::uint64_t ticks_remaining = 0;
  uint32_t halting_svc = 0;

private:
  std::uint8_t MemoryRead8(VAddr vaddr) override {
    bool error = false;
    auto value = touchHLE_cpu_read_u8(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }

  std::uint16_t MemoryRead16(VAddr vaddr) override {
    bool error = false;
    auto value = touchHLE_cpu_read_u16(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }

  std::uint32_t MemoryRead32(VAddr vaddr) override {
    bool error = false;
    auto value = touchHLE_cpu_read_u32(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }

  std::uint64_t MemoryRead64(VAddr vaddr) override {
    bool error = false;
    auto value = touchHLE_cpu_read_u64(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }

  Vector MemoryRead128(VAddr vaddr) override {
    return {MemoryRead64(vaddr), MemoryRead64(vaddr + 8)};
  }

  std::optional<std::uint32_t> MemoryReadCode(VAddr vaddr) override {
    bool error = false;
    auto value = touchHLE_cpu_read_u32(mem, vaddr, &error);
    if (error) {
      return std::nullopt;
    }
    return value;
  }

  void MemoryWrite8(VAddr vaddr, std::uint8_t value) override {
    if (touchHLE_cpu_write_u8(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }

  void MemoryWrite16(VAddr vaddr, std::uint16_t value) override {
    if (touchHLE_cpu_write_u16(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }

  void MemoryWrite32(VAddr vaddr, std::uint32_t value) override {
    if (touchHLE_cpu_write_u32(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }

  void MemoryWrite64(VAddr vaddr, std::uint64_t value) override {
    if (touchHLE_cpu_write_u64(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }

  void MemoryWrite128(VAddr vaddr, Vector value) override {
    MemoryWrite64(vaddr, value[0]);
    MemoryWrite64(vaddr + 8, value[1]);
  }

  bool MemoryWriteExclusive8(VAddr addr, std::uint8_t value, std::uint8_t expected) override {
    if (MemoryRead8(addr) != expected) return false;
    MemoryWrite8(addr, value);
    return true;
  }

  bool MemoryWriteExclusive16(VAddr addr, std::uint16_t value, std::uint16_t expected) override {
    if (MemoryRead16(addr) != expected) return false;
    MemoryWrite16(addr, value);
    return true;
  }

  bool MemoryWriteExclusive32(VAddr addr, std::uint32_t value, std::uint32_t expected) override {
    if (MemoryRead32(addr) != expected) return false;
    MemoryWrite32(addr, value);
    return true;
  }

  bool MemoryWriteExclusive64(VAddr addr, std::uint64_t value, std::uint64_t expected) override {
    if (MemoryRead64(addr) != expected) return false;
    MemoryWrite64(addr, value);
    return true;
  }

  bool MemoryWriteExclusive128(VAddr addr, Vector value, Vector expected) override {
    if (MemoryRead128(addr) != expected) return false;
    MemoryWrite128(addr, value);
    return true;
  }

  void InterpreterFallback(VAddr, size_t) override {
    abort(); // TODO
  }

  void CallSVC(std::uint32_t svc) override {
    halting_svc = svc;
    cpu->HaltExecution(HaltReasonSvc);
  }

  void ExceptionRaised(VAddr pc, Dynarmic::A64::Exception exception) override {
    if (exception == Dynarmic::A64::Exception::NoExecuteFault) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    } else if (exception == Dynarmic::A64::Exception::UnallocatedEncoding ||
               exception == Dynarmic::A64::Exception::ReservedValue) {
      cpu->HaltExecution(HaltReasonUndefinedInstruction);
    } else if (exception == Dynarmic::A64::Exception::Breakpoint) {
      cpu->HaltExecution(HaltReasonBreakpoint);
    } else {
      std::fprintf(stderr, "ExceptionRaised (A64): unexpected exception %u at %llx\n",
                   unsigned(exception), (unsigned long long)pc);
      abort();
    }
  }

  void AddTicks(std::uint64_t ticks) override {
    if (ticks > ticks_remaining) {
      ticks_remaining = 0;
      return;
    }
    ticks_remaining -= ticks;
  }

  std::uint64_t GetTicksRemaining() override {
    return ticks_remaining;
  }

  std::uint64_t GetCNTPCT() override {
    // TODO: implementar bien si hace falta
    return 0;
  }
};

class DynarmicWrapper64 {
  Environment64 env;
  std::unique_ptr<Dynarmic::A64::Jit> cpu;
  std::unique_ptr<Dynarmic::ExclusiveMonitor> mon;

public:
  DynarmicWrapper64(void * /*direct_memory_access_ptr*/, size_t /*null_page_count*/) {
    Dynarmic::A64::UserConfig user_config;
    user_config.callbacks = &env;

    mon = std::make_unique<Dynarmic::ExclusiveMonitor>(1);
    user_config.global_monitor = mon.get();

#ifndef NDEBUG
    user_config.check_halt_on_memory_access = true;
#endif

    // TODO: page_table / fastmem para 64-bit
    // Por ahora va todo por callbacks (más lento pero funciona)

    cpu = std::make_unique<Dynarmic::A64::Jit>(user_config);
    env.cpu = cpu.get();
  }

  // Acceso a registros (ejemplo)
  std::uint64_t GetReg(size_t index) const {
    return cpu->GetRegister(index);
  }

  void SetReg(size_t index, std::uint64_t value) {
    cpu->SetRegister(index, value);
  }

  std::uint64_t GetPC() const { return cpu->GetPC(); }
  void SetPC(std::uint64_t pc) { cpu->SetPC(pc); }

  std::uint64_t GetSP() const { return cpu->GetSP(); }
  void SetSP(std::uint64_t sp) { cpu->SetSP(sp); }

  void invalidate_cache_range(VAddr start, std::uint64_t size) {
    cpu->InvalidateCacheRange(start, size);
  }

  std::int32_t run_or_step(touchHLE_Mem *mem, std::uint64_t *ticks) {
    env.mem = mem;
    Dynarmic::HaltReason hr;

    if (ticks) {
      env.ticks_remaining = *ticks;
      hr = cpu->Run();
    } else {
      hr = cpu->Step();
    }

    std::int32_t res;
    if ((!hr && ticks) || (hr == Dynarmic::HaltReason::Step && !ticks)) {
      res = -1;
    } else if (Dynarmic::Has(hr, Dynarmic::HaltReason::MemoryAbort)) {
      res = -2;
    } else if (Dynarmic::Has(hr, HaltReasonUndefinedInstruction)) {
      res = -3;
    } else if (Dynarmic::Has(hr, HaltReasonBreakpoint)) {
      res = -4;
    } else if (Dynarmic::Has(hr, HaltReasonSvc)) {
      res = std::int32_t(env.halting_svc);
    } else {
      printf("unhandled halt reason (A64) %u\n", unsigned(hr));
      abort();
    }

    env.mem = nullptr;
    if (ticks) {
      *ticks = env.ticks_remaining;
    }
    return res;
  }
};

// TODO: exportar las funciones C para el wrapper 64-bit
// (touchHLE_DynarmicWrapper64_new, etc.)

} // namespace touchHLE::cpu
