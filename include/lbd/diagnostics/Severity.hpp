#pragma once

#include <iostream>
#include <string_view>
#include <lbd/utils/Macros.hpp>
#include <lbd/utils/terminal/Colors.hpp>

namespace lbd::diagnostics
{
  enum class Severity
  {
    INFO,
    ERR,
    WARNING,
    NOTE,
  };

  constexpr std::string_view severityToString(const Severity severity)
  {
    switch (severity)
    {
      case Severity::INFO: return "INFO";
      case Severity::ERR: return "ERROR";
      case Severity::WARNING: return "WARNING";
      case Severity::NOTE: return "NOTE";
    }
    unreachable_impl();
  }

  constexpr std::string_view getSeverityColor(const Severity severity, const utils::terminal::Colors &colors)
  {
    switch (severity)
    {
      case Severity::INFO: return colors.blue;
      case Severity::ERR: return colors.red;
      case Severity::WARNING: return colors.yellow;
      case Severity::NOTE: return colors.blue;
    }
    unreachable_impl();
  }

  std::ostream &operator<<(std::ostream &outputStream, const Severity &severity);
}
