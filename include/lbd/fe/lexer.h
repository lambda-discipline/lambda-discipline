#pragma once

#include <lbd/fe/loc.h>
#include <lbd/fe/token.h>
#include <string>
#include <vector>

namespace fe::lexer {
    struct Lexer {
        std::string source;
        size_t pos = 0;
        size_t row = 1;
        size_t col = 1;
        std::string filepath;

        explicit Lexer(const std::string &filepath);

        token::Token next_token();

        std::vector<token::Token> lex_all();

    private:
        [[nodiscard]] char peek() const;

        char get();

        [[nodiscard]] bool is_eof() const;

        [[nodiscard]] loc::Loc get_cur_loc() const;
    };
}
