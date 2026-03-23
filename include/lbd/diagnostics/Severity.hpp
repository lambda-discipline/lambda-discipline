#pragma once

#include <iostream>
#include <string_view>
#include <utility>
#include <lbd/utils/terminal/Colors.hpp>

namespace lbd::diagnostics
{
    enum class Severity
    {
        ERROR,
        WARNING,
        NOTE,
    };

    constexpr std::string_view severityToString(const Severity severity)
    {
        switch (severity)
        {
            case Severity::ERROR: return "ERROR";
            case Severity::WARNING: return "WARNING";
            case Severity::NOTE: return "NOTE";
        }
        std::unreachable();
    }

    constexpr std::string_view getSeverityColor(const Severity severity, const utils::terminal::Colors& colors)
    {
        switch (severity)
        {
            case Severity::ERROR: return colors.red;
            case Severity::WARNING: return colors.yellow;
            case Severity::NOTE: return colors.blue;
        }
        std::unreachable();
    }

    std::ostream& operator<<(std::ostream& outputStream, const Severity& severity);
}
