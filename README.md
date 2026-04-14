# fast-down-c

[![GitHub last commit](https://img.shields.io/github/last-commit/fast-down/fast-down-c/main)](https://github.com/fast-down/fast-down-c/commits/main)
[![Build](https://github.com/fast-down/fast-down-c/workflows/Build/badge.svg)](https://github.com/fast-down/fast-down-c/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/fast-down/fast-down-c/blob/main/LICENSE)

fast-down C 语言绑定，封装自 [fast-down-ffi](https://github.com/fast-down/ffi)，由 Rust 驱动，简洁易用。

## 编译

```bash
cargo build --release
```

编译产物在 `target/release/` 目录下：

- Linux/macOS: `libfast_down.so` 或 `libfast_down.a`
- Windows: `fast_down.dll` 或 `fast_down.lib`

```bash
cd example
gcc -O3 -o basic_download basic_download.c -I../include ../target/release/libfast_down.a
./basic_download
```

## 示例

```c
#include "fast_down.h"
#include <stdio.h>

int main() {
  const char *url = "https://example.com/test.zip";

  // 创建下载任务
  DownloadTask *task = prefetch(url, NULL, NULL);
  const char *error = download_task_get_error(task);
  if (error != NULL) {
    printf("Error: %s\n", error);
    download_task_free(&task);
    return 1;
  }

  // 获取文件信息
  UrlInfo *info = download_task_get_info(task);
  const char *filename = url_info_get_filename(info);
  uint64_t size = url_info_get_size(info);
  printf("File: %s, Size: %llu bytes\n", filename, size);

  // 开始下载
  int ret = download_task_start_to_file(task, filename, NULL, NULL);
  if (ret != 0) {
    printf("Download failed: %d\n", ret);
  }

  // 释放资源
  download_task_free(&task);
  return 0;
}
```

[查看更多示例](https://github.com/fast-down/fast-down-c/blob/main/example)
