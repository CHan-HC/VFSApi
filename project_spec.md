# vfs apis项目开发规格文档

## 1. 项目概述 (Project Overview)
- **核心目标**: 文件读写的操作例如读文件/写文件/list文件等用rust语言来编写一个rust语言的sdk，这些文件操作与华为云空间相结合，是一个本地文件系统与华为云空间融合的文件系统的相关api。
- **目标用户**: 其他rust模块可以调用这些关于文件相关的操作api。
- **系统边界**:
  - 目标产物就是rust代码对外暴露的接口read_file/write_file/list_files等api，对外不要暴露其它几个文件的内部接口，例如不要包括网络接口，log接口等。外部模块调用这个rust sdk的文件操作接口完成其自己的逻辑。

## 2. 技术栈与环境 (Tech Stack & Environment)
- **前端测试部分**: 这部分采用harmonyos应用工程作为前端测试，采用arkts语言，在页面中会对每个read/write等接口进行测试，点击对应的按钮就可以进行测试功能是否可以使用。
- **语言**: 前端测试采用harmonyos应用页面采用arkts语言进行编写，点击按钮的实现采用c++语言，代码进入napi阶段，在napi的c++文件中会调用rust暴露出来的c接口，这部分是通过一个ffi接口层，从而完成对rust相关api的测试。
- **ffi**: rust暴露出来c接口，给hamronyos工程使用采用动态库的形式，harmonyos工程的cmakelist进行编译连接
- **包管理器**: [rust部分采用标准rust工程格式]

## 3. 核心开发规范 (Core Conventions)
- **文件结构**: 所有rust源代码文件放在 `rust`目录下的src文件夹下面，前端测试harmonyos工程在vfs_apis目录了下，其中页面在index.ets文件中，napi代码在napi_init.cpp文件中。ffi的rs文件也在rust目录下的src文件夹下面。
- **代码风格**: rust api你需要设计成异步方法，以便可以让调用侧可以进行对api进行wait操作，每个api都是类似async fn read_file这种模式
- **错误处理**: 不要默默吞掉错误，每个异常都要生成错误码，让调用的rust代码能够知道异常问题，以便调用侧进行处理。
- **注释要求**: 关键步骤需要加入测试。
- **rust库引用**: 例如像网络请求返回的是json, 则需要引入serde_json这种库来使用，别自己手写流行三方库来解析json，自己手写流行三方库来解析json容易出错，而且流行三方库的解析json的库都是经过很多年验证的，所以推荐使用流行三方库来解析json。其他的逻辑也是类似，别自己创造轮子，使用流行三方库来完成。

## 4. 华为云空间相关规范
- **云空间文件管理文档地址**:`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/HMSCore-Guides/server-managing-and-searching-0000001064818926`
- **云空间查询参数**:查询参数中的containers，指查询范围，可选值drive和applicationData，你只管选择applicationData，不要选择drive。

## 5. 引用harmonyos的c api
- **rust 引用harmonyos的c api**:因为测试的rust sdk是在hamronyos应用上跑，所以需要引用harmonyos的一些api，比如rcp也就是网络能力还有log能力。因为harmonyos sdk提供了c api，所以rust是可以调用的。
- **harmonyos网络能力api介绍**:`/Applications/DevEco-Studio.app/Contents/sdk/default/hms/native/sysroot/usr/include/RemoteCommunicationKit/rcp.h`，利用这个的c api你就可以在rust语言在进行网络请求，网络请求主要用在云空间api的rest api上。对应的使用文档，请参考用法文档，`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/remote-communication-netsend-c`
- **harmonyos的log能力的头文件路径**:`/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony/native/sysroot/usr/include/hilog/log.h`，利用这个c api你就可以用rust进行log输出。对应的使用文档，请参考用法文档，`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/hilog-guidelines-ndk`


## 5. 渐进式实施计划 (Phased Implementation Plan)
[这样一个项目，你必须拆分成一个个独立的、可验证的 Phase。要求每次只完成一个 Phase。切记。]

### Phase 1: 基础脚手架
- [ ] 在rust目录下创建rust工程，里面包含cargo.toml等必要信息，创建的rust类型是一个库。
- [ ] 因为要在harmonyos设备上，需要编译rust编译成harmonyos设备上能跑的so，需要编译工具rust ohos的编译工具，只编译aarch64-unknown-linux-ohos，不要编译其它的平台。
- [ ] 编写rcp.rs文件，这个文件是调用harmonyos api的c语言的网络能力，从而完成网络请求，这个文件是调用的c api，从而完成网络请求。具体请参考上面5章节中的网络能力api介绍
，这个是一个rust内部的工具类，专门用来调用云空间的网络请求。
- [ ] 编写hilog.rs文件，这个文件是调用harmonyos api的c语言的log能力，从而完成log输出，这个文件是调用的c api，从而完成log输出。具体请参考上面5章节中的log能力api介绍，这个是一个rust内部的工具类，专门用来调用云空间的log输出。
- [ ] 编写workspace.rs文件，用来保存调用者设置的本地的工作目录，例如开发者设置本地的工作目录是/xxx/yyy，则后面的list_files/read_file等接口传递的路径，都是基于这个本地路径来处理的，例如list_files的path参数是/qqq，那么list_files实际查找的路径是/xxx/yyy/qqq。
- [ ] 在rust工程中，需要在src目录下创建lib.rs文件，这个文件是rust对外暴露的接口，也就是你后面要实现的read_file/write_file/list_files等api，这个文件是rust对外暴露的接口。
- [ ] harmonyos工程我已经创建好了，你直接使用即可，不用单独创建工程。
- [ ] 这个阶段需要创建工具类，这个地方的测试，你可以在harmonyos的index.ets页面中，通过点击按钮调用napi_init.cpp中的c接口，从而完成对rust相关api的测试，napi_init.cpp文件中会调用rust暴露出来的c接口，这部分是通过一个ffi接口层，从而完成对rust相关api的测试。通过这个地方测试工具类。

### Phase 2: 增加list.rs代码，功能是列出路径下的文件
- [ ] 对外的接口设计成异步，接口名称list_files。
- [ ] 输入参数，第一个参数是at字符串，at字符串由调用接口放传入，你不用管，第二个参数是路径path。异步的返回值的结果是list_files_result，将每一个文件的文件名称，最后修改时间，大小，来源（有可能是harmonyos的文件或者云空间的文件，通过这个地方做区分）。
- [ ] 实现逻辑，首先查找本地路径的所有文件调用rust找到文件即可，然后再访问云空间，调用云空间描述的列出文件列表的api，两者的合并结果，就是这个函数的放回结果。合并的逻辑是，如果本地和云上都有的话，则用最新的时间戳的文件为准，如果本地有，云上没有，则保留本地的文件，如果本地没有，云上有，则显示云上的文件。
- [ ] 路径要注意，如果本地路径是基于workspace.rs文件中设置的本地工作目录，例如workspace设置的路径是/xxx/yyy，则list_files的path参数是/qqq，那么list_files实际查找的路径是/xxx/yyy/qqq。
- 当前的逻辑是云空间和本地都有，这个时候云空间是时间戳比较新，因为可能是本地传上去的，所以当前显示的逻辑source是cloud

### Phase 3: 增加read.rs代码，功能是读取路径下的文件的字节数组
- 就是读的时候，判断一下本地文件以及云空间对应的文件，当前前提是本地有，云空间也有的这个场景下，你不要读到本地有，就读本地了，你应该判断一下云空间是否也有这个文件，两者你要比较一下，谁的modify时间最新，就用谁的。如果本地和云空间都有这个文件，那么读取的时候，你需要比较一下modify时间，如果本地的modify时间比云空间的modify时间大，那么读取本地的文件，如果本地的modify时间比云空间的modify时间小，那么读取云空间的文件。如果本地和云空间都没有这个文件，那么你需要返回一个错误，错误码是file_not_found。
- 如果read的时候，本地比较新，那么同步更新一下云侧的内容
- 如果read的时候，云侧比较新，读云册的时候，那么同步更新一下本地的内容
- 具体如下策略:
本地存在 + 云端存在 → 比较修改时间，使用最新的
本地存在 + 云端不存在 → 读本地
本地不存在 + 云端存在 → 从云端下载
本地不存在 + 云端不存在 → 报错

### Phase 4: 增加upload.rs代码，同步一个文件到云空间上去
- 同步一个文件到云空间上，属于内部接口，不对外暴露，给read/write等文件使用的
- 方法名称upload_file，第一个参数是at，第二个参数是文件路径
- 上传云空间文档`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/HMSCore-Guides/server-managing-and-searching-0000001064818926#section666185910356`
- 切记如果本地/xxx/yyy/1.txt,如果云空间没有xxx/yyy你还需要创建对应的文件夹，保证本地workspace路径与云空间applicationdata路径对齐就是workspace/xxx/yyy/1.txt与applicationdata/xxx/yyy/1.txt对应上
- 测试中增加一个upload的测试，也就是在inde.ets中增加一下upload的按钮测试

### Phase 5: 增加write.rs代码，写字节数组到本地文件同时需要更新云空间
- 写一个字节数组到本地的指定路径文件，写完之后，将写到本地路径上的这个文件，切记同时要同步到云空间上去
- 方法名称为write_file，第一个参数是at，第二个参数是文件路径，第三个参数是u8字节数组
- 上传文件使用upload.rs的内部接口即可
- 切记如果本地/xxx/yyy/1.txt,如果云空间没有xxx/yyy你还需要创建对应的文件夹，保证本地workspace路径与云空间applicationdata路径对齐就是workspace/xxx/yyy/1.txt与applicationdata/xxx/yyy/1.txt对应上
- 测试中增加一个write的测试，也就是在index.ets中增加一下write的按钮测试

### Phase 6: 增加rm.rs代码，能够删除一个文件
- 提供一个删除文件的接口rm_file接口，实现删除一个本地文件的接口，删除的同时如果云空间对应路径也有文件，云空间也进行删除
- 接口名称为rm_file接口，第一个参数是at，第二个参数是文件路径，返回值是布尔类型表示是否成功
- 切记如果本地/xxx/yyy/1.txt,如果云空间也有/xxx/yyy/1.txt，保证本地workspace路径与云空间applicationdata路径对齐就是workspace/xxx/yyy/1.txt与applicationdata/xxx/yyy/1.txt对应上
- 测试中增加一个rm的测试，也就是在index.ets增加一个rm的按钮测试
- 删除云空间文件的文档，你参考一下`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/HMSCore-Guides/server-managing-and-searching-0000001064818926#section1596019485273`


### Phase 7: 增加mkdir.rs代码，能够创建文件夹功能
- 提供一个叫创建文件夹接口mk_dir，实现创建本地文件夹功能，创建的同时切记，本地创建完了，你要确保云空间也有对应的文件夹
- 接口名称为mk_dir，第一个参数是at，第二个参数是dir的路径path，返回值是布尔类型表示是否成功
- 切记如果本地/xxx/yyy/qqq,如果云空间也有/xxx/yyy/qqq，保证本地workspace路径与云空间applicationdata路径对齐就是workspace/xxx/yyy/qqq与applicationdata/xxx/yyy/qqq对应上
- 测试中增加一个mkdir的测试，也就是在index.ets增加一个mkdir的按钮测试
- 创建云空间文件夹文档，你参考一下`https://r.jina.ai/https://developer.huawei.com/consumer/cn/doc/HMSCore-Guides/server-managing-and-searching-0000001064818926#section5375172818711`






