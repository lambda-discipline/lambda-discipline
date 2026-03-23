#pragma once

#include <iostream>
#include <string>
#include <lbd/cmd/Options.hpp>

namespace lbd::cmd
{
    void printHelp(std::ostream& outputStream, const std::string& programName);

    Options parseArguments(int argc, char** argv, const std::string& programName);
}
