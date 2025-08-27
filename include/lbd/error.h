#pragma once

#include <cassert>

template<typename>
inline constexpr bool always_false_v = false;

#define STATIC_ASSERT(VAL, MSG) \
	static_assert(VAL, __FILE__ ":" STRINGIFY(__LINE__) ": " MSG)

#define STATIC_ASSERT_UNREACHABLE_T(TYPE, MSG) STATIC_ASSERT((always_false_v<TYPE>), MSG)
#define STATIC_ASSERT_UNREACHABLE(MSG) STATIC_ASSERT(false, MSG)

#define STRINGIFY2(x) #x
#define STRINGIFY(x) STRINGIFY2(x)

#if defined(_MSC_VER)
#define UNREACHABLE_OP __assume(false)
#elif defined(__clang__) || defined(__GNUC__)
#define UNREACHABLE_OP __builtin_unreachable()
#elif __cplusplus >= 202300L
#include <utility>
#define UNREACHABLE_OP std::unreachable()
#else
#define UNREACHABLE_OP ((void)0)
#endif

#define UNREACHABLE(MSG) (assert(false && (MSG)), UNREACHABLE_OP)
