# Test URL Exclusion

This file tests that URLs and web addresses are excluded from spell checking.

## Common URL Formats

### HTTP/HTTPS URLs
- Visit https://example.com for more information
- Check http://localhost:8080 for the API
- Access https://api.github.com/users
- Navigate to https://www.google.com/search?q=test

### WWW Prefix
- Go to www.example.com
- Visit www.github.com/hunspell-lsp
- Check www.wikipedia.org for details
- Use www.stackoverflow.com for help

### FTP URLs
- Download from ftp://files.example.com
- Access ftp://ftp.example.org/pub/files
- Connect to ftp://user:pass@ftp.example.com

### File URLs
- Open file:///home/user/document.md
- View file:///etc/config/file.conf
- Access file:///C:/Users/user/file.txt

## Domain Patterns

### Simple Domains
- Visit example.com
- Check github.com for the code
- Go to wikipedia.org for information
- Access stackoverflow.com for help

### Subdomain Patterns
- Use api.github.com for REST endpoints
- Visit docs.example.com for documentation
- Check cdn.example.com for assets
- Access blog.example.com for articles

### Multi-level Domains
- Visit sub.example.co.uk
- Check api.v1.example.com
- Go to www.sub.example.com
- Use test.service.example.org

## Mixed Content

### URLs in Sentences
- Visit https://example.com for more details and information about the project
- Check http://localhost:8080/api/v1/users for teh endpoint
- Navigate to www.example.com/docs to see teh documentation
- Access example.com/api to get teh data

### URLs with Technical Terms
- Use `HashMap` and visit https://example.com for details
- Check the `Vec` type at docs.rust-lang.org
- Go to www.example.com to see `misspelled` examples
- Access example.com to learn about `recieve` data

## Edge Cases

### URLs in Code Blocks
```bash
curl https://api.example.com/users
wget http://files.example.com/data.zip
```

### URLs with Parameters
- Visit https://example.com?page=1&sort=name
- Check http://localhost:8080/api?id=123
- Go to https://example.com/search?q=test&lang=en
- Access https://github.com/user/repo/issues/123

### URL-like but not URLs
These should still be spell checked if they don't match URL patterns:
- The word example.com alone should be caught
- Visit domain without proper TLD should be flagged