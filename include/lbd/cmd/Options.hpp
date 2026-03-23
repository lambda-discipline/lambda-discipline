#pragma once

#include <optional>
#include <string>

namespace lbd::cmd
{
    struct Options
    {
        std::optional<std::string> filepath;
        bool showHelp = false;
        bool repl = false;
        bool debug = false;
        bool generateDocs = false;
    };
}
