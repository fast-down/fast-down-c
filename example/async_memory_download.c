// async_memory_to_file_simple.c
#include "fast_down.h"
#include <stdio.h>
#include <stdlib.h>

#ifdef _WIN32
#include <windows.h>
#define SLEEP(ms) Sleep(ms)
#else
#include <unistd.h>
#define SLEEP(ms) usleep((ms) * 1000)
#endif

// 用于在回调中传递必要数据的上下文
typedef struct {
  DownloadTask *task;
  const char *filename;
  uint8_t *data;
  uintptr_t data_len;
} DownloadContext;

// 事件回调函数
void event_callback(void *ctx_ptr, enum EventType event_type, uintptr_t id,
                    const char *message, uint64_t range_start,
                    uint64_t range_end) {
  DownloadContext *ctx = (DownloadContext *)ctx_ptr;

  switch (event_type) {
  case TaskCompleted: {
    printf("Downloaded %zu bytes into memory\n", ctx->data_len);

    // 写入磁盘
    FILE *fp = fopen(ctx->filename, "wb");
    if (fp == NULL) {
      perror("Failed to open file for writing");
      free_downloaded_data(&ctx->data, ctx->data_len);
      download_task_free(&ctx->task);
      exit(1);
    }

    size_t written = fwrite(ctx->data, 1, ctx->data_len, fp);
    fclose(fp);

    if (written != ctx->data_len) {
      printf("Write incomplete: expected %zu, wrote %zu\n", ctx->data_len,
             written);
      free_downloaded_data(&ctx->data, ctx->data_len);
      download_task_free(&ctx->task);
      exit(1);
    }

    printf("Successfully saved to %s\n", ctx->filename);

    // 释放资源后退出
    free_downloaded_data(&ctx->data, ctx->data_len);
    download_task_free(&ctx->task);
    exit(0);
  }

  case TaskFailed:
    printf("Download failed: %s\n", message ? message : "unknown error");
    if (ctx->task) {
      download_task_free(&ctx->task);
    }
    exit(1);
  default:
    break;
  }
}

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

  // 初始化上下文
  DownloadContext ctx = {
      .task = task,
      .filename = filename,
      .data = NULL,
      .data_len = 0,
  };

  // 启动异步下载到内存
  int ret = download_task_start_to_memory_async(task, &ctx.data, &ctx.data_len,
                                                event_callback, &ctx);
  if (ret != 0) {
    printf("Download to memory async start failed (error code %d)\n", ret);
    download_task_free(&task);
    return 1;
  }

  // 等待回调中的 exit 终止程序
  while (1) {
    SLEEP(1000);
  }
  return 0;
}
