#include <lbd/diagnostics/Severity.hpp>

namespace lbd::diagnostics
{
    std::ostream& operator<<(std::ostream& outputStream, const Severity& severity)
    {
        const auto colors = utils::terminal::Colors();
        outputStream << "[" << colors.bold << getSeverityColor(severity, colors) << severityToString(severity) <<
            colors.reset << "]";
        return outputStream;
    }
}
