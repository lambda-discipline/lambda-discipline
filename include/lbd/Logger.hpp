#pragma once

#include <lbd/Exceptions.hpp>
#include <lbd/source/Range.hpp>
#include <lbd/utils/terminal/Colors.hpp>
#include <lbd/utils/terminal/Terminal.hpp>

namespace lbd
{
  struct Logger
  {
    bool exitOnError = true;
    bool useColor = false;
    bool showLocation = true;

    Logger() = default;

    Logger(const bool exitOnError, const bool useColor, const bool showLocation) : exitOnError(exitOnError),
      useColor(useColor), showLocation(showLocation)
    {
      if (useColor)
      {
        utils::terminal::enableVirtualTerminal();
      }
    }

    template<typename... Arguments>
    [[noreturn]] void error(Arguments &&... arguments) const
    {
      logImplementation(utils::terminal::Colors().red, std::cout, std::forward<Arguments>(arguments)...);
      if (exitOnError)
      {
        exit(EXIT_FAILURE);
      }
      throw ControlledExit{};
    }

    template<typename... Arguments>
    void info(Arguments &&... arguments) const
    {
      logImplementation(utils::terminal::Colors().blue, std::cout, std::forward<Arguments>(arguments)...);
    }

    template<typename... Arguments>
    void debug(Arguments &&... arguments) const
    {
      logImplementation(utils::terminal::Colors().yellow, std::cout, std::forward<Arguments>(arguments)...);
    }

  private:
    template<typename... Arguments>
    void logImplementation(const std::string &color, std::ostream &stream, Arguments &&... arguments) const
    {
      if (useColor)
      {
        stream << color;
      }
      (stream << ... << std::forward<Arguments>(arguments));
      if (useColor)
      {
        stream << utils::terminal::Colors().reset;
      }
      stream << std::endl;
    }
  };
}