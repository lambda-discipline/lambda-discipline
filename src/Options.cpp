#include <lbd/Options.hpp>

namespace lbd
{
    Options::Options(const bool ownExpression, const bool forceOnEnvironmentDump, const bool debug) :
        ownExpression(ownExpression), forceOnEnvironmentDump(forceOnEnvironmentDump), debug(debug) {}

    bool Options::shouldOwnExpression() const { return ownExpression; }

    bool Options::shouldForceOnEnvironmentDump() const { return forceOnEnvironmentDump; }

    bool Options::isDebug() const { return debug; }

    void Options::toggleForceOnEnvironmentDump() { forceOnEnvironmentDump = !forceOnEnvironmentDump; }

    void Options::toggleDebug() { debug = !debug; }
}
