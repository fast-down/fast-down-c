#include <stdint.h>
#include <stdbool.h>

typedef enum EventType {
  PrefetchError = 0,
  Pulling,
  PullError,
  PullTimeout,
  PullProgress,
  Pushing,
  PushError,
  PushProgress,
  Flushing,
  FlushError,
  Finished,
  TaskCompleted,
  TaskFailed,
} EventType;

typedef struct CancellationToken CancellationToken;

typedef struct Config Config;

typedef struct DownloadTask DownloadTask;

typedef struct UrlInfo UrlInfo;

/**
 * 当下载过程中发生事件时，此回调会被调用。
 *
 * # 参数
 * - `context`: 用户注册回调时提供的自定义指针，原样传回。
 * - `event_type`: 事件类型，见 `EventType` 枚举。
 * - `id`: 关联的线程 ID（仅部分事件有效，否则为 0）。
 * - `message`: 错误消息或描述（仅部分事件有效，否则为 NULL）。
 * - `range_start`: 进度范围的起始字节，包含（仅部分事件有效有效）。
 * - `range_end`: 进度范围的结束字节，不包含（仅部分事件有效有效）。
 *
 * # 安全性
 * - `message` 指向的字符串仅在回调函数内有效，回调返回后可能被释放，调用者不应保存该指针或在其外部使用。
 */
typedef void (*EventCallback)(void *context,
                              enum EventType event_type,
                              uintptr_t id,
                              const char *message,
                              uint64_t range_start,
                              uint64_t range_end);

/**
 * 推送数据回调
 *
 * - `context`: 用户自定义指针
 * - `offset`: 数据在文件中的起始偏移量
 * - `data`: 数据指针
 * - `len`: 数据长度
 *
 * 返回值：0 成功，非 0 失败
 */
typedef int (*PushCallback)(void *context, uint64_t offset, const uint8_t *data, uintptr_t len);

/**
 * 刷新回调
 *
 * - `context`: 用户自定义指针
 *
 * 返回值：0 成功，非 0 失败
 */
typedef int (*FlushCallback)(void *context);

typedef void (*PrefetchCallback)(void *context, struct DownloadTask *task);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * 创建一个新的 `CancellationToken`
 */
struct CancellationToken *cancellation_token_new(void);

/**
 * 增加原 `CancellationToken` 的引用计数
 */
struct CancellationToken *cancellation_token_retain(const struct CancellationToken *ptr);

/**
 * 减少原 `CancellationToken` 的引用计数，若计数归零则销毁内部数据
 */
void cancellation_token_release(struct CancellationToken **ptr);

/**
 * 创建子取消令牌
 */
struct CancellationToken *cancellation_token_child(const struct CancellationToken *ptr);

/**
 * 触发取消
 */
void cancellation_token_cancel(const struct CancellationToken *ptr);

/**
 * 查询是否已取消，空指针永远返回 true
 */
bool cancellation_token_is_cancelled(const struct CancellationToken *ptr);

/**
 * 创建一个新的配置句柄，所有字段为合理的默认值
 */
struct Config *config_new(void);

/**
 * 销毁配置句柄，释放内存
 */
void config_free(struct Config **ptr);

void config_set_threads(struct Config *handle, uintptr_t threads);

void config_set_proxy(struct Config *handle, const char *proxy);

void config_insert_header(struct Config *handle, const char *key, const char *value);

bool config_remove_header(struct Config *handle, const char *key);

void config_clear_headers(struct Config *handle);

void config_set_min_chunk_size(struct Config *handle, uint64_t size);

void config_set_write_buffer_size(struct Config *handle, uintptr_t size);

void config_set_write_queue_cap(struct Config *handle, uintptr_t cap);

void config_set_retry_gap_ms(struct Config *handle, uint64_t ms);

void config_set_pull_timeout_ms(struct Config *handle, uint64_t ms);

void config_set_accept_invalid_certs(struct Config *handle, bool accept);

void config_set_accept_invalid_hostnames(struct Config *handle, bool accept);

void config_set_write_method(struct Config *handle, const char *method);

void config_set_retry_times(struct Config *handle, uintptr_t times);

bool config_add_local_address(struct Config *handle, const char *addr);

bool config_remove_local_address(struct Config *handle, const char *addr);

void config_clear_local_addresses(struct Config *handle);

void config_set_max_speculative(struct Config *handle, uintptr_t max);

void config_add_downloaded_chunk(struct Config *handle, uint64_t start, uint64_t end);

void config_clear_downloaded_chunks(struct Config *handle);

void config_set_chunk_window(struct Config *handle, uint64_t window);

/**
 * 释放任务句柄
 */
void download_task_free(struct DownloadTask **ptr);

/**
 * 彻底取消下载任务（不可恢复）
 */
void download_task_cancel(const struct DownloadTask *handle);

/**
 * 检查是否已被彻底取消，空指针永远返回 true
 */
bool download_task_is_cancelled(const struct DownloadTask *handle);

/**
 * 暂停下载任务（可恢复）
 */
void download_task_pause(const struct DownloadTask *handle);

/**
 * 检查是否处于暂停状态，空指针永远返回 true
 */
bool download_task_is_paused(const struct DownloadTask *handle);

/**
 * 获取 `UrlInfo` 句柄（禁止用 `url_info_free` 释放，这只是一个可变借用）
 */
struct UrlInfo *download_task_get_info(struct DownloadTask *handle);

/**
 * 获取 prefetch 阶段的错误信息（如果有的话）
 *
 * # 返回值
 * - 非 NULL：返回错误信息字符串
 * - NULL：无错误
 */
const char *download_task_get_error(const struct DownloadTask *handle);

/**
 * 设置/覆盖下载任务的配置（必须在 start_* 之前调用）
 */
void download_task_set_config(struct DownloadTask *handle, const struct Config *config);

/**
 * 开始下载任务写入到指定路径
 *
 * # 返回值
 * - `0` 成功
 * - `-1` 参数错误 (传入了空指针)
 * - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
 * - `-3` 下载失败
 */
int32_t download_task_start_to_file(struct DownloadTask *handle,
                                    const char *save_path,
                                    EventCallback callback,
                                    void *context);

/**
 * 开始下载任务并返回内存中的数据，释放内存需用 `free_downloaded_data` 函数
 *
 * # 返回值
 * - `0` 成功
 * - `-1` 参数错误 (传入了空指针)
 * - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
 * - `-3` 下载失败
 */
int32_t download_task_start_to_memory(struct DownloadTask *handle,
                                      uint8_t **out_data,
                                      uintptr_t *out_len,
                                      EventCallback callback,
                                      void *context);

/**
 * 释放由 `download_task_start_to_memory` 分配的内存
 */
void free_downloaded_data(uint8_t **ptr, uintptr_t len);

/**
 * 开始下载任务并使用自定义推送器
 *
 * # 返回值
 * - `0` 成功
 * - `-1` 参数错误 (传入了空指针)
 * - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
 * - `-3` 下载失败
 */
int32_t download_task_start_with_pusher(struct DownloadTask *handle,
                                        PushCallback push_cb,
                                        FlushCallback flush_cb,
                                        void *pusher_ctx,
                                        EventCallback event_cb,
                                        void *event_ctx);

int32_t download_task_start_to_file_async(struct DownloadTask *handle,
                                          const char *save_path,
                                          EventCallback callback,
                                          void *context);

int32_t download_task_start_to_memory_async(struct DownloadTask *handle,
                                            uint8_t **out_data,
                                            uintptr_t *out_len,
                                            EventCallback callback,
                                            void *context);

int32_t download_task_start_with_pusher_async(struct DownloadTask *handle,
                                              PushCallback push_cb,
                                              FlushCallback flush_cb,
                                              void *pusher_ctx,
                                              EventCallback event_cb,
                                              void *event_ctx);

/**
 * 创建下载任务
 *
 * # 参数
 * - `url`: 下载链接（UTF-8 字符串）
 * - `config`: 配置句柄（可为 NULL，使用默认配置）
 * - `token`: 取消令牌句柄（可为 NULL，内部自动创建）
 *
 * # 返回值
 * - URL 解析失败返回 NULL
 * - 其他情况返回 `DownloadTask*`
 */
struct DownloadTask *prefetch(const char *url,
                              struct Config *config,
                              struct CancellationToken *token);

/**
 * 创建下载任务
 *
 * # 参数
 * - `url`: 下载链接（UTF-8 字符串）
 * - `config`: 配置句柄（可为 NULL，使用默认配置）
 * - `token`: 取消令牌句柄（可为 NULL，内部自动创建）
 * - `callback`: 回调函数
 * - `context`: 回调上下文（可为 NULL）
 */
void prefetch_async(const char *url,
                    struct Config *config,
                    struct CancellationToken *token,
                    PrefetchCallback callback,
                    void *context);

/**
 * 释放 `UrlInfo` 句柄
 */
void url_info_free(struct UrlInfo **ptr);

uint64_t url_info_get_size(const struct UrlInfo *handle);

const char *url_info_get_raw_name(const struct UrlInfo *handle);

bool url_info_get_supports_range(const struct UrlInfo *handle);

bool url_info_get_fast_download(const struct UrlInfo *handle);

const char *url_info_get_final_url(const struct UrlInfo *handle);

const char *url_info_get_etag(const struct UrlInfo *handle);

const char *url_info_get_last_modified(const struct UrlInfo *handle);

const char *url_info_get_content_type(const struct UrlInfo *handle);

/**
 * 获取安全的文件名
 *
 * 注意悬垂指针问题，例如：
 * 1. 首次调用 `url_info_get_filename` 处理文件名，返回指针 A
 * 2. 再次调用 `url_info_get_filename` 缓存命中，返回指针 A
 * 3. 调用 `url_info_set_raw_name` 后，指针 A 指向的位置被释放，导致悬垂指针
 */
const char *url_info_get_filename(const struct UrlInfo *handle);

void url_info_set_size(struct UrlInfo *handle, uint64_t size);

void url_info_set_raw_name(struct UrlInfo *handle, const char *name);

void url_info_set_supports_range(struct UrlInfo *handle, bool value);

void url_info_set_fast_download(struct UrlInfo *handle, bool value);

void url_info_set_final_url(struct UrlInfo *handle, const char *url);

void url_info_set_etag(struct UrlInfo *handle, const char *etag);

void url_info_set_last_modified(struct UrlInfo *handle, const char *last_modified);

void url_info_set_content_type(struct UrlInfo *handle, const char *content_type);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
