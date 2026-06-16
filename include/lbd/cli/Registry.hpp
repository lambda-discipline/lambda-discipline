#pragma once

#include <memory>
#include <lbd/Context.hpp>
#include <lbd/cli/Command.hpp>

namespace lbd::cli
{
  class CommandRegistry
  {
  public:
    void add(std::unique_ptr<ICommand> command);

    void registerCommands(CLI::App &app);

    int dispatch(Context &context) const;

  private:
    std::vector<std::unique_ptr<ICommand>> commands;
    ICommand *selectedCommand = nullptr;
  };
}