#pragma once

#include <lbd/Options.hpp>
#include <lbd/source/SourceManager.hpp>

namespace lbd
{
    class Context
    {
    public:
        explicit Context(Options options);

        [[nodiscard]] source::SourceManager& getSourceManager();

        [[nodiscard]] Options& getOptions();

        // TODO: Add diagnostic emitter and renderer.

    private:
        Options options;
        source::SourceManager sourceManager;
    };
}
