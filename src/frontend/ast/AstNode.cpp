#include <lbd/frontend/ast/AstNode.hpp>

namespace lbd::frontend::ast
{
    AstNode::AstNode(const AstNodeKind kind, const source::Range range) : kind(kind), range(range) {}

    std::ostream& operator<<(std::ostream& outputStream, const AstNode& node)
    {
        node.print(outputStream, 0);
        return outputStream;
    }

    void AstNode::printIndent(std::ostream& outputStream, const size_t indent) noexcept
    {
        for (size_t i = 0; i < indent; ++i) outputStream << "    ";
    }
}
