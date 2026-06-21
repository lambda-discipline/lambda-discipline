#pragma once

#include <lbd/frontend/ast/AstNode.hpp>

namespace lbd::frontend::ast::statement
{
  class Statement;
  using StatementPtr = std::unique_ptr<Statement>;

  class Statement : public AstNode
  {
    using AstNode::AstNode;
  };
}
