#pragma once

#include "logs.h"

// TODO: Choose better namespace for this. Maybe global, context.
namespace options {
    struct Options {
        bool ownExpression = false; /// Owning Expression inside Thunk. Turned on for REPL
        bool forceOnEnvironmentDump = false;
        bool debug = false;
        logs::Logger logger;
    };
}
