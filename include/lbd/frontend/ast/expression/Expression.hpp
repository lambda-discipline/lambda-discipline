#pragma once

#include <lbd/frontend/ast/AstNode.hpp>

namespace lbd::frontend::ast::expression
{
    class Expression : public AstNode
    {
        using AstNode::AstNode;
    };
}
