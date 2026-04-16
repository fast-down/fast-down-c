#include "fast_down.h"
#include <stdio.h>
#include <stdlib.h>

// 自定义推送器上下文
typedef struct {
  FILE *fp;            // 输出文件句柄
  uint64_t total_size; // 文件总大小（用于进度显示）
  uint64_t written;    // 已写入字节数
} PusherContext;

// 推送数据回调
int push_callback(void *context, uint64_t offset, const uint8_t *data,
                  uintptr_t len) {
  PusherContext *ctx = (PusherContext *)context;

  // 定位到正确的文件偏移量
#ifdef _WIN32
  if (_fseeki64(ctx->fp, offset, SEEK_SET) != 0) {
#else
  if (fseeko(ctx->fp, offset, SEEK_SET) != 0) {
#endif
    fprintf(stderr, "Push error: seek failed at offset %llu\n", offset);
    return -1; // 返回非 0 表示失败
  }

  // 写入数据
  size_t written = fwrite(data, 1, len, ctx->fp);
  if (written != len) {
    fprintf(stderr, "Push error: write failed at offset %llu\n", offset);
    return -1;
  }

  ctx->written += len;

  // 打印进度
  if (ctx->total_size > 0) {
    double percent = (double)ctx->written / (double)ctx->total_size * 100.0;
    printf("\rProgress: %.2f%% (%llu / %llu bytes)", percent, ctx->written,
           ctx->total_size);
    fflush(stdout);
  }

  return 0;
}

// 刷新回调（下载完成时调用）
int flush_callback(void *context) {
  PusherContext *ctx = (PusherContext *)context;
  fflush(ctx->fp);
  printf("\nFlush completed.\n");
  return 0;
}

// 事件回调（可选，同步调用中仍会被触发）
void event_callback(void *ctx_ptr, enum EventType event_type, uintptr_t id,
                    const char *message, uint64_t range_start,
                    uint64_t range_end) {
  // 这里可以处理进度事件，但同步调用中返回值已经能判断成败
  switch (event_type) {
  case PushError:
    printf("\nPush error at [%llu, %llu): %s\n", range_start, range_end,
           message ? message : "unknown");
    break;
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

  // 打开输出文件
  FILE *fp = fopen(filename, "wb");
  if (fp == NULL) {
    perror("Failed to open output file");
    download_task_free(&task);
    return 1;
  }

  // 初始化 pusher 上下文
  PusherContext ctx = {
      .fp = fp,
      .total_size = size,
      .written = 0,
  };

  // 启动同步下载，使用自定义推送器（此调用会阻塞直到完成或失败）
  int ret =
      download_task_start_with_pusher(task,           // 下载任务
                                      push_callback,  // 推送回调
                                      flush_callback, // 刷新回调
                                      &ctx,           // 推送器上下文
                                      event_callback, // 事件回调（可为 NULL）
                                      &ctx            // 事件回调上下文
      );

  // 关闭文件并释放任务
  fclose(fp);
  download_task_free(&task);

  if (ret == 0) {
    printf("\nDownload completed successfully!\n");
    return 0;
  } else {
    printf("\nDownload failed with error code %d\n", ret);
    return 1;
  }
}
