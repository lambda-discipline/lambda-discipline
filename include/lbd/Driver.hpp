#pragma once

#include <lbd/Context.hpp>
#include <lbd/runtime/interpreter.h>
#include <lbd/source/Buffer.hpp>

namespace lbd
{
    class Driver
    {
    public:
        explicit Driver(Context& context) noexcept;

        [[nodiscard]] runtime::Result run(source::BufferId bufferId) const noexcept;

    private:
        Context& context;
    };
}
