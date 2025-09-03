#pragma once
#include "logs.h"

namespace options {
    struct Options {
        bool own_expr = false; /// Owning Expression inside Thunk. Turned on for REPL
        bool force_on_env_dump = false;
        bool debug = false;
        logs::Logger logger;
    };
}
