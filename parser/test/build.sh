#!/usr/bin/env sh
# -parameters is that method arguments store the name
javac -parameters Everything.java
javac -parameters Thrower.java
javac -parameters Variants.java
javac -parameters Constants.java
javac -parameters Super.java
# -g ist with Debug info
javac -parameters -g LocalVariableTable.java
