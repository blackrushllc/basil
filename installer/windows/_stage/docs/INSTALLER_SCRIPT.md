```
curl -fsSL https://basrun.com/install.sh -o /tmp/basrun-install.sh && bash /tmp/basrun-install.sh --verbose 2>&1 | tee /tmp/basrun-install.log

Doesn't work:
curl -fsSL https://basrun.com/install.sh | sudo bash


In addition to the regular linux build script, do this too:

Make basilc-safe for portability:
cargo build --release -p basilc --target x86_64-unknown-linux-musl --features portable,obj-safe
Copy to install dir:
cp /var/www/basil/target/x86_64-unknown-linux-musl/release/basilc 1.0.0/linux/basilc-safe
Rebuild json:
root@YORE-ARC:/var/www/basrun/downloads# ./make_json.sh
root@YORE-ARC:/var/www/basrun/downloads# chmod 775 *

After running the install script, publish basil.cgi:

root@ip-172-31-9-213:/var/www# install -m 0755 /usr/local/bin/basilc-safe /usr/lib/cgi-bin/basil.cgi
root@ip-172-31-9-213:/var/www# ll /usr/lib/cgi-bin
total 84868
drwxr-xr-x  2 root root     4096 Feb 15 16:20 ./
drwxr-xr-x 86 root root     4096 Jul 12  2025 ../
-rwxr-xr-x  1 root root 35932640 Feb 15 16:20 basil.cgi*
-rwxr-xr-x  1 root root 50959272 Feb 15 16:17 basilc*

vhost for basil scripting:

<VirtualHost *:80>
ServerName basrun.com

    # Where your site lives
    DocumentRoot /var/www/basrun

    <Directory "/var/www/basrun">
        Options +ExecCGI +FollowSymLinks
        AllowOverride None
        Require all granted

        # "/" -> /var/www/brb/cgi/index.basil
        DirectoryIndex cgi/index.basil

        # Allow extra path segments after scripts (PATH_INFO)
        AcceptPathInfo On
    </Directory>

    # Map /guide and everything under it to guide.basil
    #AliasMatch ^/guide($|/.*) /var/www/brb/cgi/guide.basil

    # Map /reference and everything under it to reference.basil
    #AliasMatch ^/reference($|/.*) /var/www/brb/cgi/reference.basil

    # Classic CGI dir (for basil.cgi)
    ScriptAlias /cgi-bin/ /usr/lib/cgi-bin/
    <Directory "/usr/lib/cgi-bin/">
        Options +ExecCGI
        AllowOverride None
        Require all granted
    </Directory>

    # Basil handler: any .basil file is run through basil.cgi
    AddHandler basil-script .basil
    Action basil-script /cgi-bin/basil.cgi

    ServerAdmin webmaster@localhost
    ErrorLog ${APACHE_LOG_DIR}/error.log
    CustomLog ${APACHE_LOG_DIR}/access.log combined

</VirtualHost>



or



<VirtualHost *:80>
ServerName blackrushbasic.com

    # Where your site lives
    DocumentRoot /var/www/brb

    <Directory "/var/www/brb">
        Options +ExecCGI +FollowSymLinks
        AllowOverride None
        Require all granted

        # "/" -> /var/www/brb/cgi/index.basil
        DirectoryIndex cgi/index.basil

        # Allow extra path segments after scripts (PATH_INFO)
        AcceptPathInfo On
    </Directory>

    # Map /guide and everything under it to guide.basil
    AliasMatch ^/guide($|/.*) /var/www/brb/cgi/guide.basil

    # Map /reference and everything under it to reference.basil
    AliasMatch ^/reference($|/.*) /var/www/brb/cgi/reference.basil

    # Classic CGI dir (for basil.cgi)
    ScriptAlias /cgi-bin/ /usr/lib/cgi-bin/
    <Directory "/usr/lib/cgi-bin/">
        Options +ExecCGI
        AllowOverride None
        Require all granted
    </Directory>

    # Basil handler: any .basil file is run through basil.cgi
    AddHandler basil-script .basil
    Action basil-script /cgi-bin/basil.cgi

    ServerAdmin webmaster@localhost
    ErrorLog ${APACHE_LOG_DIR}/error.log
    CustomLog ${APACHE_LOG_DIR}/access.log combined

</VirtualHost>










downloads
+---dist
+---linux
¦       basilc
¦       basilc-bmx
¦       basilc-daw
¦       basilc-naked
¦       bcc
¦       bcc-bmx
¦       bcc-daw
¦       bcc-naked
¦       basilc-safe
¦
+---mac
¦       basilc
¦       basilc-bmx
¦       basilc-daw
¦       basilc-naked
¦       bcc
¦       bcc-bmx
¦       bcc-daw
¦       bcc-naked
¦
+---windows
        basil-serve.exe
        Basil.msi
        basilc-bmx.exe
        basilc-daw.exe
        basilc-naked.exe
        basilc.exe
        bcc-bmx.exe
        bcc-daw.exe
        bcc-naked.exe
        bcc.exe
```