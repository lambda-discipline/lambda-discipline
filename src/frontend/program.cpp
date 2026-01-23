#include <lbd/frontend/program.h>

namespace frontend {
    std::ostream &operator<<(std::ostream &outputStream, const Program &program) {
        for (const AstNode &node: program.astNodes) {
            outputStream << node;
        }
        return outputStream;
    }
}
