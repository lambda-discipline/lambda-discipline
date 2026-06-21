#pragma once

#include <lbd/frontend/ast/AstNode.hpp>

namespace lbd::frontend::ast::expression
{
  class Expression;
  using ExpressionPtr = std::unique_ptr<Expression>;

  class Expression : public AstNode
  {
    using AstNode::AstNode;
  };
}
