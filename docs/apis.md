# 使用参考
## 1.set_workspace接口
这个接口的目的是，设置本地的工作空间路径与云空间的applicationdata路径对齐。

这样就能本地路径和云空间的路径有了映射关系，方便根据一个相对路径，这个相对路径与云空间路径的映射关系。

举例：以端侧是harmonyos app为例，如果workspace的路径是/data/storage/el2/base/files/mywork，如果read_file的路径则是abc/cde，

本地路径：/data/storage/el2/base/files/mywork/abc/cde

云空间路径：/application/abc/cde

后续针对于这个目录的读写等文件接口，都是按照上面的映射关系进行处理，例如读一个文件本地没有，这样就知道去哪个云空间路径下去取了。
### 接口定义

```rust
pub async fn set_workspace(path: &str) -> VfsResult<()>
```

### 参数

path：工作空间目录的绝对路径。

### 返回值

`Ok(())` 表示设置成功。

### 使用示例

```rust
use vfs_apis::set_workspace;

set_workspace("/data/storage/el2/base/files/applicationdata").await.unwrap();
```

> **注意：** 应用启动时调用一次即可，后续所有VFS接口的相对路径都会基于此工作空间进行解析。

---

## 2.list_files接口

列出当前对应路径下的所有文件，例如列出/abc/cde下的文件，从端读/data/storage/el2/base/files/mywork/abc/cde路径下的文件，再从云空间读取/application/abc/cde路径下的文件，进行合并返回结果

### 接口
```rust
pub async fn list_files(at: &str, path: &str) -> VfsResult<ListFilesResult>
```
### 参数
at：云空间at字符串

path：路径名称

### 返回值
```rust
#[derive(Debug, Clone)]
pub struct ListFilesResult {
    pub files: Vec<FileInfo>,
    pub error_code: ErrorCode,       // 0 = Success
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,         // 文件名或目录名
    pub modified_time: u64,   // 最后修改时间，Unix 秒级时间戳
    pub size: u64,            // 文件大小（字节），目录固定为 0
    pub source: u32,          // 来源：1 = Local（本地），2 = Cloud（云端）
    pub is_directory: bool,   // true = 目录，false = 普通文件
}
```
### 使用示例

```rust
use vfs_apis::{set_workspace, list_files};

let result = list_files("xxxxxx", "/xxx/yyy").await.unwrap();
for f in &result.files {
    let kind = if f.is_directory { "DIR" } else { "FILE" };
    println!("  {} {}  {}B  modified={}  source={}",
        kind, f.name, f.size, f.modified_time, f.source
    );
}
}
```

---

## 3.read_file接口

读取一个文件，在本地或者云空间找一个最新时间戳的文件进行读取返回字节数组。
### 接口

```rust
pub async fn read_file(at: &str, path: &str) -> VfsResult<ReadFileResult>
```

### 参数

at：云空间at字符串

path：相对于工作空间的文件路径

### 返回值

```rust
#[derive(Debug, Clone)]
pub struct ReadFileResult {
    pub content: Vec<u8>,             // 文件内容（字节数组）
    pub error_code: ErrorCode,        // 0 = Success
    pub error_message: Option<String>,
}
```

### 使用示例

```rust
use vfs_apis::{set_workspace, read_file, ErrorCode};

let result = read_file("xxxxxx", "/docs/readme.txt").await.unwrap();
if result.error_code == ErrorCode::Success {
    let text = String::from_utf8_lossy(&result.content);
    println!("文件内容: {}", text);
}
```

---

## 4.write_file接口

写一个字节数组到本地文件中，并且同步到云空间对应的文件中

### 接口

```rust
pub async fn write_file(at: &str, path: &str, content: &[u8]) -> VfsResult<()>
```

### 参数

at：云空间at字符串

path：相对于工作空间的文件路径

content：内容

### 返回值

`Ok(())` 表示本地写入成功。

### 使用示例

```rust
use vfs_apis::{set_workspace, write};

write::write_file("xxxxxx", "/docs/hello.txt", b"Hello, World!").await.unwrap();

let data = r#"{"key": "value"}"#;
write::write_file("xxxxxx", "/data/config.json", data.as_bytes()).await.unwrap();
```

---

## 5.mk_dir接口

创建一个文件夹并且同步到云空间

### 接口

```rust
pub async fn mk_dir(at: &str, path: &str) -> VfsResult<bool>
```

### 参数

at：云空间at字符串

path：相对于工作空间的目录路径

### 返回值

- `Ok(true)` — 创建成功
- `Err(VfsError)` — 失败

### 使用示例

```rust
use vfs_apis::{set_workspace, mkdir};

let created = mkdir::mk_dir("xxxxxx", "/docs/subdir").await.unwrap();
println!("创建结果: {}", created);
```

---

## 6.rm_file接口

删除一个文件接口

### 接口

```rust
pub async fn rm_file(at: &str, path: &str) -> VfsResult<bool>
```

### 参数

at：云空间at字符串

path：相对于工作空间的文件路径

### 返回值

- `Ok(true)` — 成功

### 使用示例

```rust
use vfs_apis::{set_workspace, rm};

let deleted = rm::rm_file("xxxxxx", "/docs/hello.txt").await.unwrap();
println!("删除结果: {}", deleted);
```

---

## 7.upload_file接口

上传一个本地文件到云空间

### 接口

```rust
pub async fn upload_file(at: &str, path: &str) -> VfsResult<()>
```

### 参数

at：云空间at字符串

path：相对于工作空间的文件路径。文件必须在本地已存在。

### 返回值

`Ok(())` 表示上传成功。使用 multipart upload 方式上传到华为云盘，会自动创建云端缺失的父目录。

### 使用示例

```rust
use vfs_apis::{set_workspace, upload};

upload::upload_file("xxxxxx", "/docs/report.pdf").await.unwrap();
```
---
