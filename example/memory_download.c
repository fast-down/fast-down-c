#include "fast_down.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
  const char *url = "https://mirrors.tuna.tsinghua.edu.cn/archlinux/iso/"
                    "2026.02.01/archlinux-x86_64.iso";

  // 创建配置
  Config *config = config_new();
  config_insert_header(
      config, "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 Edg/145.0.0.0");
  config_set_proxy(config, "no");

  // 预取文件信息
  DownloadTask *task = prefetch(url, config, NULL);
  config_free(&config);
  if (task == NULL) {
    printf("Failed to parse URL\n");
    return 1;
  }
  const char *error = download_task_get_error(task);
  if (error != NULL) {
    printf("Prefetch error: %s\n", error);
    download_task_free(&task);
    return 1;
  }

  // 获取文件名和大小
  UrlInfo *info = download_task_get_info(task);
  const char *filename = url_info_get_filename(info);
  uint64_t size = url_info_get_size(info);
  printf("File: %s, Size: %llu bytes\n", filename, size);

  // 下载到内存
  uint8_t *data = NULL;
  uintptr_t data_len = 0;

  int ret = download_task_start_to_memory(task, &data, &data_len, NULL, NULL);
  if (ret != 0) {
    printf("Download to memory failed (error code %d)\n", ret);
    download_task_free(&task);
    return 1;
  }
  printf("Downloaded %zu bytes into memory\n", data_len);

  // 写入磁盘
  FILE *fp = fopen(filename, "wb");
  if (fp == NULL) {
    perror("Failed to open file for writing");
    free_downloaded_data(&data, data_len);
    download_task_free(&task);
    return 1;
  }

  size_t written = fwrite(data, 1, data_len, fp);
  fclose(fp);

  if (written != data_len) {
    printf("Write incomplete: expected %zu, wrote %zu\n", data_len, written);
  } else {
    printf("Successfully saved to %s\n", filename);
  }

  // 释放资源
  free_downloaded_data(&data, data_len);
  download_task_free(&task);
  return 0;
}
