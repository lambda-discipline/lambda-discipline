#pragma once

#include <lbd/frontend/ast/AstNodeKind.hpp>
#include <lbd/source/Range.hpp>

namespace lbd::frontend::ast
{
  class AstNode
  {
  public:
    AstNode(AstNodeKind kind, source::Range range);

    virtual ~AstNode() = default;

    virtual void print(std::ostream &outputStream, size_t indent) const noexcept = 0;

    friend std::ostream &operator<<(std::ostream &outputStream, const AstNode &node);

  private:
    const AstNodeKind kind;
    const source::Range range;

  protected:
    static void printIndent(std::ostream &outputStream, size_t indent) noexcept;
  };
}