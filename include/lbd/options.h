#pragma once
#include "logs.h"

namespace options {
    struct Options {
        bool ownExpression = false; /// Owning Expression inside Thunk. Turned on for REPL
        bool forceOnEnvironmentDump = false;
        bool debug = false;
        logs::Logger logger;
    };
}
