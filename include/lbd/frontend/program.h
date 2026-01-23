#pragma once

#include <lbd/frontend/ast.h>
#include <vector>

namespace frontend {
    struct Program {
        std::vector<AstNode> astNodes;

        friend std::ostream &operator<<(std::ostream &outputStream, const Program &program);
    };
}
