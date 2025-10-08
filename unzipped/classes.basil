REM Demo: Using CLASS to instantiate and interact with a class instance

DIM user@ AS CLASS("my_class.basil");

REM Access and modify a public variable
PRINTLN "Initial description:", user@.Description$;
LET user@.Description$ = "These are my favorite users.";
PRINTLN "Updated description:", user@.Description$;

REM Call public functions
user@.AddUser("Erik");
user@.AddUser("Junie");
user@.AddUser("ChatGPT");

PRINTLN "User count:", user@.CountMyUsers%();
