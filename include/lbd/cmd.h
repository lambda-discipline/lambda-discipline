#pragma once

#include <iostream>
#include <optional>
#include <string>

namespace cmd {
    struct Options {
        std::optional<std::string> filepath;
        bool showHelp = false;
        bool repl = false;
        bool debug = false;
    };

    void printHelp(std::ostream &os, const std::string &programName);

    Options parseArguments(int argc, char **argv, const std::string &programName);
}
