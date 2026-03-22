#include  <lbd/Context.hpp>

namespace lbd
{
    Context::Context(const Options options) : options(options) {}

    source::SourceManager& Context::getSourceManager() { return sourceManager; }

    Options& Context::getOptions() { return options; }
}
