#pragma once

namespace lbd::utils::terminal
{
    struct Colors
    {
        const char* reset = "";
        const char* bold = "";
        const char* red = "";
        const char* green = "";
        const char* yellow = "";
        const char* blue = "";
        const char* cyan = "";

        [[nodiscard]] Colors();
    };
}
