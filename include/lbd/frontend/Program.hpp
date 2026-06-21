#pragma once

#include <vector>
#include <lbd/frontend/ast/AstNode.hpp>

namespace lbd::frontend
{
  class Program
  {
  public:
    explicit Program(std::vector<ast::AstNodePtr> astNodePtrs);

    friend std::ostream &operator<<(std::ostream &outputStream, const Program &program);

    [[nodiscard]] std::vector<ast::AstNodePtr> &getAstNodePtrs() noexcept;

    [[nodiscard]] const std::vector<ast::AstNodePtr> &getAstNodePtrs() const noexcept;

  private:
    std::vector<ast::AstNodePtr> astNodePtrs;
  };
}
