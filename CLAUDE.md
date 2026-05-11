# 🤖 Claude 项目指南

## 📌 项目概述
这是一个提供一套虚拟文件系统接口的rust sdk的工程。
主要目标是为其它的rust库提供对应read/write/list/rm等等这样一类文件操作系统的接口。其它的rust库可以以来这个虚拟文件系统的相关接口进行读写操作。
这些虚拟的文件操作接口目标是将本地工作空间目录以及华为云空间目录进行融合操作，不只是操作本地文件。
同时这个项目的前端的测试页面采用了一个harmonyos app来进行测试，harmonyos app中的c接口会调用rust接口进行测试，但是真实环境是另外一个rust库来调用这个虚拟文件操作系统接口。具体spec请参考：project_spec.md文件

## 💻 技术栈
- **核心框架:** 每个接口都是融合本地文件操作以及云空间文件操作融合的逻辑，每个操作的逻辑有不同。
- **语言:** 文件操作接口采用rust进行编写，测试工程采用arkts以及c进行编写，通过ffi由harmonyos app调用到对应的rust sdk。
- **测试:** 测试采用的是harmonyos的页面进行测试。

## 🏗️ 架构与目录结构
- `rust/src/` - 各个操作的接口源代码，例如read/rm/upload等等，以及内部的工具代码例如rcp网络代码等等 
- `vfs_apis` - 前端harmonyos工程目录 
- `/vfs_apis/entry/src/ets` - 前端harmonyos工程页面
- `/vfs_apis/entry/src/cpp` - c++层调用rust sdk逻辑
- `build_rust.sh` - 编译rust脚本工程，并且将编译成harmonyos的so，拷贝到harmonysos app下面的libs

## 千万注意的事情
- 不要提交harmony中OAuthManager.ts文件到远程仓库