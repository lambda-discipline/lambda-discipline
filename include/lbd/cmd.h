#pragma once

#include <iostream>
#include <optional>
#include <string>

namespace cmd {
    struct Options {
        std::optional<std::string> filepath;
        bool show_help = false;
        bool repl = false;
        bool debug = false;
    };

    void print_help(std::ostream &os, const std::string &program_name);

    Options parse_args(int argc, char **argv, const std::string &program_name);
}
