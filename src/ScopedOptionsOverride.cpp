#include <lbd/ScopedOptionsOverride.hpp>

namespace lbd
{
  ScopedOptionsOverride::ScopedOptionsOverride(Context &context)
    : context(context), oldOptions(context.getOptions()) {}

  ScopedOptionsOverride::~ScopedOptionsOverride() { context.getOptions() = oldOptions; }

  Options &ScopedOptionsOverride::get() const { return context.getOptions(); }
}