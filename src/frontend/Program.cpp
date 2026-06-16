#include <lbd/frontend/Program.hpp>

namespace lbd::frontend
{
  Program::Program(std::vector<std::unique_ptr<ast::AstNode>> nodes) : astNodes(std::move(nodes)) {}

  std::ostream &operator<<(std::ostream &outputStream, const Program &program)
  {
    for (const auto &node: program.astNodes)
    {
      outputStream << *node;
    }
    return outputStream;
  }
}