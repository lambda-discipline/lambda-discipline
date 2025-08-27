#include <lbd/fe/loc.h>
#include <lbd/fe/parser.h>
#include <lbd/string_escape.h>

namespace fe::parser {
    Parser::Parser(const std::vector<token::Token> &tokens) {
        size_t i = 0;
        program = ast::Program{build_ast(tokens, i)};
    }

    template<typename T>
    void Parser::assert_token(const std::vector<token::Token> &tokens, size_t &i) {
        if (const token::Token &cur_token = tokens[i]; !std::holds_alternative<T>(cur_token.typ)) {
            std::cerr << cur_token.loc << ": syntax error: expected "
                    << token::to_string<T>()
                    << ", got " << cur_token.to_string()
                    << std::endl;
            std::exit(EXIT_FAILURE);
        }
    }

    template<typename T>
    void Parser::assert_n_eat(const std::vector<token::Token> &tokens, size_t &i) {
        assert_token<T>(tokens, i);
        ++i;
    }

    ast::IdenAstNode Parser::eat_iden(const std::vector<token::Token> &tokens, size_t &i) {
        assert_token<token::Iden>(tokens, i);
        auto [value] = std::get<token::Iden>(tokens[i].typ);
        const loc::Loc loc = tokens[i].loc;
        ++i;
        return ast::IdenAstNode{value, loc};
    }

    intp::types::PrimitiveType Parser::eat_primitive_type_name(const std::vector<token::Token> &tokens, size_t &i) {
        assert_token<token::Iden>(tokens, i);
        auto [value] = std::get<token::Iden>(tokens[i].typ);
        loc::Loc loc = tokens[i].loc;
        ++i;
        if (value == "Float") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Float};
        }
        if (value == "Str") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Str};
        }
        if (value == "Any") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Any};
        }
        return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Custom, std::move(value)};
    }

    intp::types::Type Parser::parse_type(const std::vector<token::Token> &tokens, size_t &i) {
        intp::types::PrimitiveType typ = eat_primitive_type_name(tokens, i);
        std::vector types{typ};
        while (std::holds_alternative<token::Arrow>(tokens[i].typ)) {
            assert_n_eat<token::Arrow>(tokens, i);
            types.push_back(eat_primitive_type_name(tokens, i));
        }
        if (types.size() == 1) {
            return typ;
        }
        // Construct Right Associative AST
        intp::types::Type c_typ = types.back();
        for (int typ_i = static_cast<int>(types.size()) - 2; typ_i >= 0; --typ_i) {
            auto next_c_typ = std::make_shared<intp::types::CompoundType>();
            next_c_typ->l_type = types[typ_i];
            next_c_typ->r_type = c_typ;
            c_typ = next_c_typ;
        }
        return c_typ;
    }

    ast::Expression Parser::parse_expression(const std::vector<token::Token> &tokens, size_t &i) {
        const auto &tok = tokens[i];
        const loc::Loc loc = tok.loc;
        return std::visit([&]<typename T0>(T0 &&) {
            using T = std::decay_t<T0>;
            if (std::is_same_v<T, token::Iden>) {
                ++i;
                const auto value = std::get<token::Iden>(tok.typ).value;
                return ast::Expression(ast::IdenAstNode{value, loc});
            }
            if (std::is_same_v<T, token::String>) {
                ++i;
                const auto value = unescape_string(std::get<token::String>(tok.typ).value);
                return ast::Expression(ast::StringAstNode{value, loc});
            }
            if (std::is_same_v<T, token::Float>) {
                ++i;
                const auto value = std::get<token::Float>(tok.typ).value;
                return ast::Expression(ast::FloatAstNode{value, loc});
            }
            if (std::is_same_v<T, token::BackwardSlash>) {
                return ast::Expression(parse_lambda_expression(tokens, i));
            }
            if (std::is_same_v<T, token::OpenParen>) {
                return ast::Expression(parse_function_application(tokens, i));
            }
            std::cerr << loc << ": syntax error: unexpected token "
                    << tok.to_string()
                    << std::endl;
            std::exit(EXIT_FAILURE);
        }, tok.typ);
    }

    ast::LambdaExpression Parser::parse_lambda_expression(const std::vector<token::Token> &tokens, size_t &i) {
        loc::Loc loc = tokens[i].loc;
        assert_n_eat<token::BackwardSlash>(tokens, i);
        ast::IdenAstNode arg = eat_iden(tokens, i);
        assert_n_eat<token::Colon>(tokens, i);
        intp::types::Type arg_type = parse_type(tokens, i);
        assert_n_eat<token::Dot>(tokens, i);
        ast::Expression expr = parse_expression(tokens, i);
        return ast::LambdaExpression{arg, arg_type, std::make_unique<ast::Expression>(std::move(expr)), loc};
    }

    ast::FunctionApplication Parser::parse_function_application(const std::vector<token::Token> &tokens, size_t &i) {
        const loc::Loc loc = tokens[i].loc;
        assert_n_eat<token::OpenParen>(tokens, i);
        const ast::IdenAstNode fn_name = eat_iden(tokens, i);
        std::vector<std::unique_ptr<ast::Expression> > args;
        while (!std::holds_alternative<token::CloseParen>(tokens[i].typ)) {
            args.push_back(std::make_unique<ast::Expression>(std::move(parse_expression(tokens, i))));
        }
        assert_n_eat<token::CloseParen>(tokens, i);
        return ast::FunctionApplication{fn_name, std::move(args), loc};
    }

    ast::DefAstNode Parser::parse_def_ast_node(const std::vector<token::Token> &tokens, size_t &i) {
        loc::Loc loc = tokens[i].loc;
        ast::IdenAstNode def_name = eat_iden(tokens, i);
        assert_n_eat<token::Colon>(tokens, i);
        intp::types::Type typ = parse_type(tokens, i);
        assert_n_eat<token::Equal>(tokens, i);
        ast::Expression expr = parse_expression(tokens, i);
        return ast::DefAstNode{def_name, typ, std::move(expr), loc};
    }

    std::vector<ast::AstNode> Parser::build_ast(const std::vector<token::Token> &tokens, size_t &i) {
        std::vector<ast::AstNode> nodes;
        while (!std::holds_alternative<token::Eof>(tokens[i].typ)) {
            const token::Token &tok = tokens[i];
            nodes.push_back(std::visit([&]<typename T0>(T0 &&) {
                using T = std::decay_t<T0>;
                if (std::is_same_v<T, token::Iden>) {
                    return ast::AstNode{std::move(parse_def_ast_node(tokens, i))};
                }
                if (std::is_same_v<T, token::String>
                    || std::is_same_v<T, token::Float>
                    || std::is_same_v<T, token::BackwardSlash>
                    || std::is_same_v<T, token::OpenParen>) {
                    return ast::AstNode{std::move(parse_expression(tokens, i))};
                }
                std::cerr << tok.loc << ": syntax error: unexpected token "
                        << tok.to_string()
                        << std::endl;
                std::exit(EXIT_FAILURE);
            }, tok.typ));
        }
        return nodes;
    }
}
