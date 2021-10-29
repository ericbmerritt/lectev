Lectev
======

Lectev is a tool for interacting with Jira. It contains several things that the,
as a manager, find useful.

Features
---------

Time In Status
~~~~~~~~~~~~~~

Provides the time that an issue has spent in a particular status. The user has
to provide a mapping of Jira statuses to Lectev statuses.

Development
-----------

The `dev.sh` script in the root of the project provides the main entry point for
development. That script makes use of a number of commands, so the author highly
recommends that you make use of the Nix_, Direnv_ and `Nix Direnv`_. Otherwise
you can review the `denv.sh` script and install the various commands that it
uses.

You can also just use the cargo commands directly. You will loose the benefit of
the supporting infrastructure, but if all you want to do is build the system it
works.

.. _Nix: https://nixos.org
.. _Direnv: https://direnv.net
.. _Nix Direnv: https://github.com/nix-community/nix-direnv
