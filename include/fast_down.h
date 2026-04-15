#pragma once

#include "fast_down_ffi.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/**
 * 合并进度区间
 *
 * ## 参数
 *
 * - `entries_ptr`: 指向区间数组指针的指针（会通过 realloc 更新）
 * - `count`: 当前区间个数
 * - `cap_ptr`: 指向当前数组容量的指针（以区间对数为单位）
 * - `new_start`: 新区间起始
 * - `new_end`: 新区间结束
 *
 * ## 返回值
 *
 * 合并后的区间个数，若失败返回 (size_t)-1
 */
static inline size_t merge_progress(uint64_t **entries_ptr, size_t count,
                                    size_t *cap_ptr, uint64_t new_start,
                                    uint64_t new_end) {
  if (!entries_ptr || !cap_ptr)
    return (size_t)-1;

  uint64_t *entries = *entries_ptr;
  size_t cap = *cap_ptr;

  if (count >= cap) {
    size_t new_cap = (cap == 0) ? 4 : cap * 2;
    uint64_t *new_entries = realloc(entries, new_cap * 2 * sizeof(uint64_t));
    if (!new_entries)
      return (size_t)-1;
    *entries_ptr = entries = new_entries;
    *cap_ptr = new_cap;
  }

  size_t lo = 0, hi = count;
  while (lo < hi) {
    size_t mid = lo + (hi - lo) / 2;
    if (entries[mid * 2 + 1] < new_start)
      lo = mid + 1;
    else
      hi = mid;
  }
  size_t i = lo;

  if (i < count && entries[i * 2] <= new_start && entries[i * 2 + 1] >= new_end)
    return count;

  size_t j = i;
  uint64_t merged_start = new_start;
  uint64_t merged_end = new_end;
  while (j < count && entries[j * 2] <= merged_end) {
    if (entries[j * 2] < merged_start)
      merged_start = entries[j * 2];
    if (entries[j * 2 + 1] > merged_end)
      merged_end = entries[j * 2 + 1];
    j++;
  }

  if (j == i) {
    memmove(&entries[(i + 1) * 2], &entries[i * 2],
            (count - i) * 2 * sizeof(uint64_t));
    entries[i * 2] = new_start;
    entries[i * 2 + 1] = new_end;
    return count + 1;
  } else {
    entries[i * 2] = merged_start;
    entries[i * 2 + 1] = merged_end;
    size_t removed = j - i - 1;
    if (removed > 0) {
      memmove(&entries[(i + 1) * 2], &entries[j * 2],
              (count - j) * 2 * sizeof(uint64_t));
    }
    return count - removed;
  }
}
