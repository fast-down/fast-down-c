#include "fast_down.h"
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

// 进度上下文
typedef struct ProgressContext {
  uint64_t *entries;   // 区间数组，每两个 uint64_t 表示 [start, end)
  size_t count;        // 当前区间个数
  size_t cap;          // 数组容量（以区间对数为单位）
  uint64_t total_size; // 文件总大小
} ProgressContext;

// 初始化进度上下文
ProgressContext *progress_context_new(uint64_t total_size) {
  ProgressContext *ctx = malloc(sizeof(ProgressContext));
  ctx->entries = NULL;
  ctx->count = 0;
  ctx->cap = 0;
  ctx->total_size = total_size;
  return ctx;
}

// 释放进度上下文
void progress_context_free(ProgressContext *ctx) {
  if (ctx) {
    free(ctx->entries);
    free(ctx);
  }
}

// 计算已下载字节总数
uint64_t progress_get_downloaded(const ProgressContext *ctx) {
  uint64_t downloaded = 0;
  for (size_t i = 0; i < ctx->count; i++) {
    uint64_t start = ctx->entries[i * 2];
    uint64_t end = ctx->entries[i * 2 + 1];
    downloaded += (end - start);
  }
  return downloaded;
}

// 事件回调函数
void event_callback(void *ctx_ptr, enum EventType event_type, uintptr_t id,
                    const char *message, uint64_t range_start,
                    uint64_t range_end) {
  ProgressContext *ctx = (ProgressContext *)ctx_ptr;

  switch (event_type) {
  case PullProgress:
  case PushProgress: {
    // 合并新区间
    size_t new_count = merge_progress(&ctx->entries, ctx->count, &ctx->cap,
                                      range_start, range_end);
    if (new_count != (size_t)-1) {
      ctx->count = new_count;
      // 计算并打印进度百分比
      uint64_t downloaded = progress_get_downloaded(ctx);
      if (ctx->total_size > 0) {
        double percent = (double)downloaded / (double)ctx->total_size * 100.0;
        printf("\rProgress: %.2f%% (%llu / %llu bytes)", percent, downloaded,
               ctx->total_size);
        fflush(stdout);
      }
    }
    break;
  }
  case TaskCompleted:
    printf("\nDownload completed!\n");
    break;
  case TaskFailed:
    printf("\nDownload failed: %s\n", message ? message : "unknown error");
    break;
  default:
    break;
  }
}

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
  config_free(&config);
  if (task == NULL) {
    printf("Url parse failed\n");
    return 1;
  }
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
  printf("File: %s, Size: %" PRIu64 " bytes\n", filename, size);

  // 创建进度上下文
  ProgressContext *progress_ctx = progress_context_new(size);

  // 开始下载
  int ret =
      download_task_start_to_file(task, filename, event_callback, progress_ctx);
  if (ret != 0) {
    printf("Download failed: %d\n", ret);
  }

  // 释放资源
  progress_context_free(progress_ctx);
  download_task_free(&task);
  return 0;
}
