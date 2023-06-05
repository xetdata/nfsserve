NFS Server
==========
Ihis is an incomplete implementation. 
More things in Mount Protocol and NFS Protocol has to be implemented.

This provides an NFS Server. You simply need to implement the vfs::NFSFileSystem
trait. See demofs.rs for an example and bin/main.rs for how to actually start
a service. The interface generally not difficult to implement; demanding mainly
the ability to associate every file system object (directory/file) with a 64-bit
ID. Directory listing can be a bit complicated due to the pagination requirements.


Relevant RFCs
=============
 - XDR is the message format: RFC 1014. https://datatracker.ietf.org/doc/html/rfc1014
 - SUN RPC is the RPC wire format: RFC 1057 https://datatracker.ietf.org/doc/html/rfc1057
 - NFS is at RFC 1813 https://datatracker.ietf.org/doc/html/rfc1813
 - NFS Mount Protocol is at RFC 1813 Appendix I. https://datatracker.ietf.org/doc/html/rfc1813#appendix-I
 - PortMapper is at RFC 1057 Appendix A https://datatracker.ietf.org/doc/html/rfc1057#appendix-A

Messaging
=========
The basic way a message works is:
1. We read a collection of fragments off a TCP stream 
(a 4 byte length header followed by a bunch of bytes)
2. We assemble the fragments into a record
3. The Record is of a SUN RPC message type.
4. A message tells us 3 pieces of information,
     - The RPC Program (just an integer denoting
      a protocol "class". For instance NFS protocol is 100003, the Portmapper protocol is 100000).
     - The version of the RPC program (ex: 3 = NFSv3, 4 = NFSv4, etc)
     - The method invoked (Which NFS method to call) (See for instance nfs.rs top comment for the list)
5. Continuing to decode the message will give us the arguments of the method
6. And we take the method response, wrap it around a record and return it. 

Basic Source Layout
===================
 - context.rs: A connection context object that is passed around containing
 connection information, VFS information, etc.
 - xdr.rs: Serialization / Deserialization routines for XDR structures
 - tcp.rs: Main TCP handling entry point
 - rpcwire.rs: Reads and write RPC messages from a TCP socket and performs outer 
               most RPC message decoding, redirecting to NFS/Mount/Portmapper 
               implementations as needed.
 - rpc.rs: The structure of a RPC call and reply. All XDR encoded.
 - portmap.rs/portmap\_handlers.rs: The XDR structures required by the Portmapper protocol and the Portmapper RPC handlers.
 - mount.rs/mount\_handlers.rs: The XDR structures required by the Mount protocol and the Mount RPC handlers.
 - nfs.rs/nfs\_handlers.rs: The XDR structures required by the NFS protocol and the NFS RPC handlers.


Portmapper
----------
First, lets get portmapper out of the way. This is a *very* old mechanism which
is rarely used anymore. The portmapper is a daemon which runs on a machine running
on port 111. When NFS, or other RPC services start, they register with the 
portmapper service with the port they are listening on (Say NFS on 2049). 
Then when another machine wants to connect to NFS, they first ask the port mapper
on 111 to ask about which port NFS is listening on, then connects to the returned 
port.

We do not strictly need to implement this protocol as this is pretty much
unused these days (NFSv4 does not use the portmapper for instance). If `-o port` and `-o mountport`
are specified, Linux and Mac's builtin NFS client do not need it either.
But this was useful for debugging and testing as libnfs seems to require a
portmapper, but it annoyingly hardcodes it to 111. I modified the source to
change it to 12000 for testing and implemented the one `PMAPPROC_GETPORT`
method so I can test with libnfs.


NFS Basics
==========
The way NFS works is that every file system object (dir/file/symlink) has 2
ways in which it can be addressed:

1. `fileid3: u64` . A 64-bit integer. Equivalent to an inode number.
2. `nfs_fh3`: A variable opaque object up to 64 bytes long.

Basically anytime the client tries to access any information about an object,
it needs an `nfs_fh3`. The purpose of the `nfs_fh3` serves 2 purposes:

 - Allow server to cache additional query information in the handle that may exceed
 64-bit. For instance if the server has multiple exports on different disk volumes,
 I may need a few more bits to identify the disk volume.
 - Allow client to identify when server has "restarted" and thus client has to
 clear all caches. the `nfs_fh3` handle should contain a token that is unique
 to when the NFS server first started up which allows the server to check that
 the handle is still valid. If the server has restarted, all previous handles
 will therefore be "expired" and any usage of them should trigger a handle expiry
 error informing the clients to expunge all caches.


However, the only way to obtain an `nfs_fh3` for a file is via directory traversal.
i.e. There is a lookup method 
`LOOKUP(directory's handle, filename of file/dir in directory)` 
which returns the handle for the filename.

For instance to get the handle of a file "dir/a.txt", I first need the handle
for the directory "dir/", then query `LOOKUP(handle, "a.txt")`.

The question is then, how do I get my first handle? That is what the MOUNT
protocol addresses.

Mount
-----
The MOUNT protocol provides a list of "exports", (in the simplest case. Just "/")
and the client will request to MNT("/") which will return the handle of this 
root directory.

Normally the server can and do maintain a list of mounts which can be queried,
and really the client can UMNT (unmount) as well.  But in our case we
only implement MNT and EXPORT which suffices. NFS clients generally
ignore the return message of UMNT as there is really nothing the
client can do on a UMNT failure. As such our Mount protocol implementation
is entirely stateless.

NFS
---
The NFS protocol itself is pretty straightforward with most annoyances
due to handling of the XDR messaging format (in paticular with optional,
lists, etc).

What is nice is that the design of NFS is completely stateless. It is mostly
sit down and implement all the methods that are hit and test them against a 
client.
