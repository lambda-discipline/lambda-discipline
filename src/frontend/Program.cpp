#include <lbd/frontend/Program.hpp>

namespace lbd::frontend
{
  Program::Program(std::vector<ast::AstNodePtr> astNodePtrs) : astNodePtrs(std::move(astNodePtrs)) {}

  std::ostream &operator<<(std::ostream &outputStream, const Program &program)
  {
    for (const auto &node: program.astNodePtrs) outputStream << *node;
    return outputStream;
  }

  std::vector<ast::AstNodePtr> &Program::getAstNodePtrs() noexcept { return astNodePtrs; }

  const std::vector<ast::AstNodePtr> &Program::getAstNodePtrs() const noexcept { return astNodePtrs; }
}