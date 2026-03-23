#pragma once

namespace lbd
{
    // TODO: Ensure these fields are private.
    struct Options
    {
        /// Owning Expression inside Thunk. Turned on for REPL
        bool ownExpression = false;
        bool forceOnEnvironmentDump = false;
        bool debug = false;
    };
}
