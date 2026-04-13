#include "fast_down.h"
#include <stdio.h>

int main() {
  const char *url = "https://mirrors.tuna.tsinghua.edu.cn/archlinux/iso/"
                    "2026.02.01/archlinux-x86_64.iso";

  // 创建任务配置
  Config *config = config_new();
  config_insert_header(
      config, "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 Edg/145.0.0.0");
  config_set_proxy(config, "no");

  // 创建下载任务
  DownloadTask *task = prefetch(url, config, NULL);
  const char *error = download_task_get_error(task);
  if (error != NULL) {
    printf("Error: %s\n", error);
    download_task_free(&task);
    config_free(&config);
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
  config_free(&config);
  return 0;
}
