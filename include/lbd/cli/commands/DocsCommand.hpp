#pragma once

#include <lbd/cli/Command.hpp>

namespace lbd::cli
{
  class DocsCommand final : public ICommand
  {
  public:
    void configure(CLI::App &command) override;

    int execute(Context &context) override;
  };
}
