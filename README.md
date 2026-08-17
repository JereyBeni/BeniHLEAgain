# RayHLE

**RayHLE** is a 64-bit iOS HLE project based on HyperHLE, focused on improving compatibility with legacy iOS applications and games.

## Features

* 🧩 **64-bit support**
* 🎞️ **Correct frame sequencing**

  * Frames are presented in the correct order (`1 → 2 → 3 → 4`)
  * Fixes HyperHLE's out-of-order presentation issues
* 🎙️ **Microphone support**
* 📷 **Camera support**
* 📱 Designed for running legacy iOS software through HLE

## Why RayHLE?

Some legacy iOS applications can behave incorrectly under HyperHLE due to differences in frame presentation and missing hardware APIs.

For example:

```text
Real iOS:   1 → 2 → 3 → 4 → 5 → 6
HyperHLE:   4 → 1 → 3 → 2 → 6 → 5
RayHLE:     1 → 2 → 3 → 4 → 5 → 6
```

RayHLE aims to preserve the behavior expected by the original applications while extending HyperHLE with modern compatibility improvements.

## Status

🚧 **Work in progress**

RayHLE is currently under development. Features and compatibility may change as development continues.

## Credits

RayHLE is based on the work of the **HyperHLE** project and its contributors.

Made for the preservation of legacy iOS software. 🫡
