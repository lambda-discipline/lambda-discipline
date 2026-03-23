#include <lbd/utils/terminal/Colors.hpp>
#include <lbd/utils/terminal/Terminal.hpp>

namespace lbd::utils::terminal
{
    Colors::Colors()
    {
        if (!isTTY())
        {
            return;
        }
        reset = "\033[0m";
        bold = "\033[1m";
        red = "\033[31m";
        green = "\033[32m";
        yellow = "\033[33m";
        blue = "\033[34m";
        cyan = "\033[36m";
    }
}
