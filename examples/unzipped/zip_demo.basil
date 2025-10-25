println "ZIP demo";

// Compress a single file
zip_compress_file("zip_demo.basil", "onefile.zip", "zip_demo.basil");
println "Created onefile.zip";

// Compress a directory recursively
zip_compress_dir("../examples", "examples.zip");
println "Created examples.zip";

// List entries
let list$ = zip_list$("examples.zip");
println "Entries in examples.zip:";
println list$;

// Extract everything
zip_extract_all("examples.zip", "unzipped");
println "Extracted to ./unzipped";


// Also list entries using ZIP_ARRAY$ (string array)
dim entries$(0);
LET entries$ = zip_array$("examples.zip");
println "Entries in examples.zip (array):";
for each e$ in entries$
  println " - ", e$;
next
