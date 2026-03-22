#pragma once

#include <lbd/logs.h>

namespace lbd
{
    class Options
    {
    public:
        explicit Options(bool ownExpression = false, bool forceOnEnvironmentDump = false, bool debug = false);

        [[nodiscard]] bool shouldOwnExpression() const;

        [[nodiscard]] bool shouldForceOnEnvironmentDump() const;

        [[nodiscard]] bool isDebug() const;

        void toggleForceOnEnvironmentDump();

        void toggleDebug();

        // TODO: Instead use diagnostic emitter and renderer.
        Logger logger;

    private:
        bool ownExpression; /// Owning Expression inside Thunk. Turned on for REPL
        bool forceOnEnvironmentDump;
        bool debug;
    };
}
