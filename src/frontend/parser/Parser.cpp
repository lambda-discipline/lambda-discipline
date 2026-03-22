#include <filesystem>
#include <lbd/frontend/location.h>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/frontend/parser/Parser.hpp>

namespace lbd::frontend::parser
{
    Parser::Parser(lexer::Lexer& lexer, Context& context) : context(context), lexer(lexer) {}

    std::vector<std::unique_ptr<ast::AstNode>> Parser::parse() const
    {
        std::vector<std::unique_ptr<ast::AstNode>> astNodes;
        while (lexer.hasNext())
        {
            // std::cout << "Parsing: " << lexer.peek() << std::endl;
            switch (lexer.peek().getKind())
            {
                case token::TokenKind::IDENTIFIER:
                    {
                        if (lexer.peek().getLexeme() == "use")
                        {
                            lexer.advance();
                            assertToken(token::TokenKind::STRING);
                            std::string path(lexer.next().getLexeme());
                            for (auto includedAstNodes = parseFile(path); auto& astNode : includedAstNodes)
                            {
                                astNodes.emplace_back(std::move(astNode));
                            }
                        }
                        else
                        {
                            astNodes.push_back(std::make_unique<ast::AstNode>(parseDefinitionAstNode()));
                        }
                        break;
                    }
                case token::TokenKind::STRING:
                case token::TokenKind::NUMBER:
                case token::TokenKind::BACKWARD_SLASH:
                case token::TokenKind::OPEN_PARENTHESIS:
                    {
                        astNodes.push_back(std::make_unique<ast::AstNode>(ast::AstNode{parseExpression()}));
                        break;
                    }
                case token::TokenKind::END_OF_FILE: break;
                default:
                    context.getOptions().logger.error(lexer.peek().getRange().getBegin(),
                                                      "syntax error: unexpected token ",
                                                      token::tokenKindToString(lexer.peek().getKind()));
            }
        }

        return astNodes;
    }

    void Parser::assertToken(const token::TokenKind expectedKind) const
    {
        if (lexer.peek().getKind() != expectedKind)
        {
            context.getOptions().logger.error(lexer.peek().getRange().getBegin(), "syntax error: expected ",
                                              token::tokenKindToString(expectedKind), ", got ",
                                              token::tokenKindToString(lexer.peek().getKind()));
        }
    }

    void Parser::assertAndAdvance(const token::TokenKind expectedKind) const
    {
        assertToken(expectedKind);
        lexer.advance();
    }

    std::vector<std::unique_ptr<ast::AstNode>> Parser::parseFile(const std::string& path) const
    {
        // Circular Dependency or Duplicate Load.
        if (context.getSourceManager().isFilePathLoaded(path)) { return {}; }

        source::Buffer buffer(context.getSourceManager().loadFile(path), context.getSourceManager());
        lexer::Lexer innerLexer(buffer, context);
        Parser parser(innerLexer, context);
        return parser.parse();
    }

    ast::DefinitionAstNode Parser::parseDefinitionAstNode() const
    {
        // TODO: In future change this to range.
        source::Location location = lexer.peek().getRange().getBegin();
        ast::IdentifierAstNode definitionName = parseIdentifierAstNode();
        assertAndAdvance(token::TokenKind::COLON);
        interpreter::type::Type definitionType = parseType();
        assertAndAdvance(token::TokenKind::ASSIGNMENT);
        ast::Expression expression = parseExpression();
        return ast::DefinitionAstNode{definitionName, definitionType, std::move(expression), location};
    }

    ast::IdentifierAstNode Parser::parseIdentifierAstNode() const
    {
        assertToken(token::TokenKind::IDENTIFIER);
        const source::Location location = lexer.peek().getRange().getBegin();
        const std::string_view name = lexer.next().getLexeme();
        return ast::IdentifierAstNode{std::string(name), location};
    }

    ast::StringAstNode Parser::parseStringAstNode() const
    {
        assertToken(token::TokenKind::STRING);
        const source::Location location = lexer.peek().getRange().getBegin();
        const std::string_view value = lexer.next().getLexeme();
        // TODO: Relook whether we need to escape the string or not.
        return ast::StringAstNode{std::string(value), location};
    }

    ast::FloatAstNode Parser::parseNumberAstNode() const
    {
        assertToken(token::TokenKind::NUMBER);
        const source::Location location = lexer.peek().getRange().getBegin();
        const double value = std::stod(std::string(lexer.next().getLexeme()));
        return ast::FloatAstNode{value, location};
    }

    // TODO: Remove primitive and compound type altogether and create TypeAstNode.
    interpreter::type::PrimitiveType Parser::parsePrimitiveTypeName() const
    {
        assertToken(token::TokenKind::IDENTIFIER);
        const std::string_view name = lexer.next().getLexeme();
        // TODO: Perform this check through interpreter::type package.
        if (name == "Float")
        {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Float};
        }
        if (name == "Str")
        {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::String};
        }
        if (name == "Any")
        {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Any};
        }
        return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Custom, std::string(name)};
    }

    interpreter::type::Type Parser::parseType() const
    {
        interpreter::type::PrimitiveType typ = parsePrimitiveTypeName();
        std::vector types{typ};
        while (lexer.hasNext() && lexer.peek().getKind() == token::TokenKind::ARROW)
        {
            assertAndAdvance(token::TokenKind::ARROW);
            types.push_back(parsePrimitiveTypeName());
        }
        if (types.size() == 1) return typ;
        // Construct Right Associative AST
        interpreter::type::Type currentType = types.back();
        for (int typeIndex = static_cast<int>(types.size()) - 2; typeIndex >= 0; --typeIndex)
        {
            auto nextCompoundType = std::make_shared<interpreter::type::CompoundType>();
            nextCompoundType->leftType = types[typeIndex];
            nextCompoundType->rightType = currentType;
            currentType = nextCompoundType;
        }
        return currentType;
    }

    ast::Expression Parser::parseExpression() const
    {
        const source::Location location = lexer.peek().getRange().getBegin();
        switch (lexer.peek().getKind())
        {
            case token::TokenKind::IDENTIFIER: return ast::Expression(parseIdentifierAstNode());
            case token::TokenKind::STRING: return ast::Expression(parseStringAstNode());
            case token::TokenKind::NUMBER: return ast::Expression(parseNumberAstNode());
            case token::TokenKind::BACKWARD_SLASH: return ast::Expression(parseLambdaExpression());
            case token::TokenKind::OPEN_PARENTHESIS: return ast::Expression(parseFunctionApplication());
            default:
                context.getOptions().logger.error(location, "syntax error: unexpected token ",
                                                  token::tokenKindToString(lexer.peek().getKind()));
        }
    }

    ast::LambdaExpression Parser::parseLambdaExpression() const
    {
        const source::Location location = lexer.peek().getRange().getBegin();
        assertAndAdvance(token::TokenKind::BACKWARD_SLASH);
        ast::IdentifierAstNode argument = parseIdentifierAstNode();
        assertAndAdvance(token::TokenKind::COLON);
        interpreter::type::Type argumentType = parseType();
        assertAndAdvance(token::TokenKind::DOT);
        ast::Expression expression = parseExpression();
        return ast::LambdaExpression{
            argument, argumentType, std::make_unique<ast::Expression>(std::move(expression)), location
        };
    }

    ast::FunctionApplication Parser::parseFunctionApplication() const
    {
        const source::Location location = lexer.peek().getRange().getBegin();
        assertAndAdvance(token::TokenKind::OPEN_PARENTHESIS);
        const ast::IdentifierAstNode functionName = parseIdentifierAstNode();
        std::vector<std::unique_ptr<ast::Expression>> arguments;
        while (lexer.peek().getKind() != token::TokenKind::CLOSE_PARENTHESIS)
        {
            arguments.push_back(std::make_unique<ast::Expression>(std::move(parseExpression())));
        }
        assertAndAdvance(token::TokenKind::CLOSE_PARENTHESIS);
        return ast::FunctionApplication{functionName, std::move(arguments), location};
    }
}
