#include <fstream>
#include <sstream>
#include <lbd/source/FileLoader.hpp>

namespace lbd::source
{
    std::string loadFileFromDisk(const std::string& path)
    {
        const std::ifstream inputFileStream(path);
        std::stringstream stringStream;
        stringStream << inputFileStream.rdbuf();
        return stringStream.str();
    }
}
