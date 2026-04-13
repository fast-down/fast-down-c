#pragma once

#include "fast_down_ffi.h"

#include <stdint.h>
#include <string.h>

/**
 * 合并进度区间，两两成对，表示一个左闭右开的区间
 */
static inline size_t merge_progress(uint64_t *entries, size_t count,
                                    uint64_t new_start, uint64_t new_end) {
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
