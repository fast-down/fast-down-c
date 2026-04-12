#include "../include/fast_down.h"
#include <stdint.h>
#include <stdio.h>

/* Event callback: print progress */
static void on_event(void *context, EventType event_type, uintptr_t id,
                     const char *message, uint64_t range_start,
                     uint64_t range_end) {
  uint64_t *total_size = (uint64_t *)context;
  (void)range_start;
  (void)message;
  switch (event_type) {
  case PushProgress:
    if (*total_size > 0) {
      double pct = (double)range_end / (double)(*total_size) * 100.0;
      printf("\r  Progress: %.1f%%  (%llu / %llu bytes)", pct,
             (unsigned long long)range_end, (unsigned long long)*total_size);
      fflush(stdout);
    }
    break;
  case Flushing:
    printf("\n  Flushing to disk...\n");
    break;
  case Finished:
    printf("  Thread %zu finished\n", (size_t)id);
    break;
  case PullError:
  case PushError:
  case FlushError:
    if (message)
      fprintf(stderr, "\n  Error: %s\n", message);
    break;
  default:
    break;
  }
}

int main(void) {
  /* Arch Linux ISO（清华镜像，~900MB） */
  const char *url = "https://mirrors.tuna.tsinghua.edu.cn/archlinux/iso/"
                    "2026.02.01/archlinux-x86_64.iso";
  const char *save_path = "download/archlinux-x86_64.iso";

  printf("fast-down-c Simple Download Example\n");
  printf("URL: %s\n\n", url);

  /* 1. Create config (optional) */
  Config *cfg = config_new();
  config_set_threads(cfg, 4);
  config_set_proxy(cfg, "no");
  config_insert_header(
      cfg, "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, "
      "like Gecko) Chrome/145.0.0.0 Safari/537.36");

  /* 2. Prefetch: get file metadata and create download task */
  printf("Fetching file info...\n");
  DownloadTask *task = prefetch(url, cfg, NULL);
  config_free(&cfg);

  const char *err = download_task_get_error(task);
  if (err != NULL) {
    printf("Error: %s\n", err);
    download_task_free(&task);
    return 1;
  }

  /* 3. Read file metadata */
  UrlInfo *info = download_task_get_info(task);
  uint64_t total_size = url_info_get_size(info);
  const char *filename = url_info_get_filename(info);
  int supports_range = url_info_get_supports_range(info);
  int fast_download = url_info_get_fast_download(info);

  printf("Filename:     %s\n", filename ? filename : "(unknown)");
  printf("File size:    %llu bytes (%.2f KiB)\n",
         (unsigned long long)total_size, (double)total_size / 1024.0);
  printf("Range resume: %s\n", supports_range ? "yes" : "no");
  printf("Fast mode:    %s\n\n", fast_download ? "yes" : "no");

  /* 4. Start download */
  printf("Downloading -> %s\n", save_path);
  int32_t ret =
      download_task_start_to_file(task, save_path, on_event, &total_size);

  printf("\n");
  switch (ret) {
  case 0:
    printf("Download succeeded!\n");
    break;
  case -1:
    fprintf(stderr, "Argument error (null pointer)\n");
    break;
  case -2:
    fprintf(stderr, "Task already running\n");
    break;
  case -3:
    fprintf(stderr, "Download failed\n");
    break;
  default:
    fprintf(stderr, "Unknown error: %d\n", ret);
    break;
  }

  /* 5. Release task handle */
  download_task_free(&task);

  return ret == 0 ? 0 : 1;
}
