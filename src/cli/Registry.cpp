#include <lbd/cli/Registry.hpp>

namespace lbd::cli
{
  void CommandRegistry::add(std::unique_ptr<ICommand> command)
  {
    commands.push_back(std::move(command));
  }

  void CommandRegistry::registerCommands(CLI::App &app)
  {
    for (auto &command: commands)
    {
      auto *subcommand = app.add_subcommand("");
      command->configure(*subcommand);

      subcommand->callback([this, ptr = command.get()]
      {
        selectedCommand = ptr;
      });
    }
  }

  int CommandRegistry::dispatch(Context &context) const
  {
    if (!selectedCommand) return 0;
    return selectedCommand->execute(context);
  }
}
