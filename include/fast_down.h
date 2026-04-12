#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct CancellationToken CancellationToken;

typedef struct Config Config;

typedef struct UrlInfo UrlInfo;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * 创建一个新的 CancellationToken
 */
struct CancellationToken *cancellation_token_new(void);

/**
 * 增加原 CancellationToken 的引用计数
 */
struct CancellationToken *cancellation_token_retain(const struct CancellationToken *ptr);

/**
 * 减少原 CancellationToken 的引用计数，若计数归零则销毁内部数据
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
 * 查询是否已取消
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
 * 释放 UrlInfo 句柄
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
