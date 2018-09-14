pub mod client;
pub mod command;
pub mod message;
pub mod prefix;

pub(crate) fn is_valid_nick(_nick: &str) -> bool {
    /*
   No specific character set is specified. The protocol is based on a
   set of codes which are composed of eight (8) bits, making up an
   octet.  Each message may be composed of any number of these octets;
   however, some octet values are used for control codes, which act as
   message delimiters.

   Regardless of being an 8-bit protocol, the delimiters and keywords
   are such that protocol is mostly usable from US-ASCII terminal and a
   telnet connection.

   Because of IRC's Scandinavian origin, the characters {}|^ are
   considered to be the lower case equivalents of the characters []\~,
   respectively. This is a critical issue when determining the
   equivalence of two nicknames or channel names.
   */

    true
}
