# key-transparency


## Installation instructions

### Dependencies
* We know this code compiles with rustc 1.59.0.
* Install `tmux` and `virtualenv`: for mac you can do this using Homebrew and on linux using apt-get. 

### Running the code in a virtual environment
* Run the virtual environment
```
virtualenv venv
source venv/bin/activate
```
* The remaining steps are within the virtual environment.
* Run `pip install -r requirements.txt`.

###
Once everything is setup with the vitual env,
* `fab start` to start the VMs on aws
* `fab stop` to turn off the connections so as to put the experiment testbed to sleep.
* `fab remote` to run the experiment itself. Change params as needed in `fab remote`.

[work in progress]
