#include <lbd/Error.hpp>
#include <lbd/frontend/ast/Ast.hpp>
#include <lbd/utils/StringEscaping.hpp>

namespace lbd::frontend::ast
{
  static void printIndent(std::ostream &stream, const size_t indent)
  {
    for (size_t i = 0; i < indent; ++i)
    {
      stream << "    ";
    }
  }

  std::ostream &operator<<(std::ostream &outputStream, const IdentifierAstNode &node)
  {
    return outputStream << node.value;
  }

  std::ostream &operator<<(std::ostream &outputStream, const StringAstNode &node)
  {
    return outputStream << "\"" << escapeString(node.value) << "\"";
  }

  std::ostream &operator<<(std::ostream &outputStream, const FloatAstNode &node)
  {
    return outputStream << node.value;
  }

  void LambdaExpression::print(std::ostream &outputStream, const size_t indent) const
  {
    printIndent(outputStream, indent);
    outputStream << "\\" << argument << ": " << argumentType << "." << std::endl;
    expression->print(outputStream, indent + 1);
  }

  std::ostream &operator<<(std::ostream &outputStream, const LambdaExpression &lambdaExpression)
  {
    lambdaExpression.print(outputStream, 0);
    return outputStream;
  }

  void FunctionApplication::print(std::ostream &outputStream, const size_t indent) const
  {
    printIndent(outputStream, indent);
    outputStream << "(" << functionName;
    for (auto &argument: arguments)
    {
      outputStream << " " << *argument;
    }
    outputStream << ")";
  }

  std::ostream &operator<<(std::ostream &outputStream, const FunctionApplication &functionApplication)
  {
    functionApplication.print(outputStream, 0);
    return outputStream;
  }

  Expression::Expression(IdentifierAstNode value) : value(std::move(value)) {}

  Expression::Expression(StringAstNode value) : value(std::move(value)) {}

  Expression::Expression(FloatAstNode value) : value(value) {}

  Expression::Expression(LambdaExpression value) : value(std::move(value)) {}

  Expression::Expression(FunctionApplication value) : value(std::move(value)) {}

  void Expression::print(std::ostream &outputStream, size_t indent) const
  {
    std::visit([&]<typename T0>(T0 &&argument)
    {
      using T = std::decay_t<T0>;
      if constexpr (std::is_same_v<T, IdentifierAstNode>
                    || std::is_same_v<T, StringAstNode>
                    || std::is_same_v<T, FloatAstNode>)
      {
        printIndent(outputStream, indent);
        outputStream << argument;
      } else if constexpr (std::is_same_v<T, LambdaExpression> || std::is_same_v<T, FunctionApplication>)
      {
        argument.print(outputStream, indent);
      } else
      {
        STATIC_ASSERT_UNREACHABLE_T(T, "unhandled expression");
      }
    }, value);
  }

  std::ostream &operator<<(std::ostream &outputStream, const Expression &expression)
  {
    expression.print(outputStream, 0);
    return outputStream;
  }

  source::Location Expression::getLocation() const
  {
    return std::visit([&](auto &&arg)
    {
      return arg.location;
    }, value);
  }


  source::Range Expression::getRange() const noexcept
  {
    return std::visit([&](auto &&arg)
    {
      // TODO: Fix this when the expression starts to store Range instead of Location.
      return source::Range(arg.location, arg.location);
    }, value);
  }

  void DefinitionAstNode::print(std::ostream &outputStream, const size_t indent) const
  {
    printIndent(outputStream, indent);
    outputStream << "def " << definitionName << ": " << definitionType << " = " << expression;
  }

  std::ostream &operator<<(std::ostream &outputStream, const DefinitionAstNode &node)
  {
    node.print(outputStream, 0);
    return outputStream;
  }

  void AstNode::print(std::ostream &outputStream, size_t indent) const
  {
    std::visit([&](auto &&arg)
    {
      arg.print(outputStream, indent);
    }, value);
    outputStream << std::endl;
  }

  std::ostream &operator<<(std::ostream &outputStream, const AstNode &node)
  {
    node.print(outputStream, 0);
    return outputStream;
  }
}