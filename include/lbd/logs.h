#pragma once

#include <optional>
#include <lbd/exceptions.h>
#include <lbd/source/Range.hpp>
#include <lbd/utils/terminal.h>

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
                enableVirtualTerminal();
            }
        }

        template <typename... Arguments>
        [[noreturn]] void error(const std::optional<source::Location> location, Arguments&&... arguments) const
        {
            if (useColor)
            {
                std::cerr << colors::RED;
            }
            if (showLocation && location.has_value())
            {
                if (location->getFileId().has_value()) std::cerr << location->getFileId().has_value() << ":";
                std::cerr << location->getRow() << ":" << location->getColumn() << ": ";
            }
            (std::cerr << ... << std::forward<Arguments>(arguments));
            if (useColor)
            {
                std::cerr << colors::RESET;
            }
            std::cerr << std::endl;
            if (exitOnError)
            {
                exit(EXIT_FAILURE);
            }
            throw ControlledExit{};
        }

        template <typename... Arguments>
        void info(Arguments&&... arguments) const
        {
            logImplementation(colors::BLUE, std::cout, std::forward<Arguments>(arguments)...);
        }

        template <typename... Arguments>
        void debug(Arguments&&... arguments) const
        {
            logImplementation(colors::YELLOW, std::cout, std::forward<Arguments>(arguments)...);
        }

    private:
        template <typename... Arguments>
        void logImplementation(const std::string& color, std::ostream& stream, Arguments&&... arguments) const
        {
            if (useColor)
            {
                stream << color;
            }
            (stream << ... << std::forward<Arguments>(arguments));
            if (useColor)
            {
                stream << colors::RESET;
            }
            stream << std::endl;
        }
    };
}
