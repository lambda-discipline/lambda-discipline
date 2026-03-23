#pragma once

#include <lbd/Context.hpp>

namespace lbd
{
    class ScopedOptionsOverride
    {
    public:
        explicit ScopedOptionsOverride(Context& context);

        ~ScopedOptionsOverride();

        Options& get() const;

    private:
        Context& context;
        Options oldOptions;
    };
}
