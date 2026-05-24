/// Strips common HTML tags and decodes HTML entities from a string.
/// Handles the tags typically emitted by ANN and AniList.
pub fn clean_html(s: &str) -> String {
    s.replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("<p>", "")
        .replace("<i>", "*")
        .replace("</i>", "*")
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}