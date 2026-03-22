#pragma once

#include <iostream>
#include <vector>
#include <lbd/frontend/ast/Ast.hpp>

namespace lbd::frontend
{
    // TODO: Remove this later.
    class Program
    {
    public:
        explicit Program(std::vector<std::unique_ptr<ast::AstNode>> nodes);

        friend std::ostream& operator<<(std::ostream& outputStream, const Program& program);

        std::vector<std::unique_ptr<ast::AstNode>> astNodes;
    };
}
