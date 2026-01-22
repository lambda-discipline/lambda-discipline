#pragma once

#include <lbd/fe/loc.h>
#include <lbd/fe/token.h>
#include <lbd/options.h>
#include <string>
#include <vector>

namespace fe::lexer {
    struct FromFile {
    };

    struct FromRepl {
    };

    struct Lexer {
        Lexer(const std::string &filepath, FromFile, options::Options options_ = {});

        Lexer(std::string str, FromRepl, options::Options options_ = {});

        token::Token nextToken();

        // TODO: Find a better name for this.
        std::vector<token::Token> lexAll();

    private:
        std::string source;
        size_t position = 0;
        size_t row = 1;
        size_t col = 1;
        std::string filepath;

        [[nodiscard]] char peek() const;

        char get();

        [[nodiscard]] bool isEof() const;

        [[nodiscard]] loc::Loc getCurrentLocation() const;
    };
}
