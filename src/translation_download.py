import hashlib
import re
import time
import urllib.error
import urllib.request


def verified_archive(root, direction, url, size, expected, label="M2M100", suffix=".zip"):
    """Keep verified archives and resumable partials across installation attempts."""
    directory = root / "downloads"
    directory.mkdir(exist_ok=True)
    archive = directory / f"{direction}{suffix}"
    partial = directory / f"{direction}{suffix}.part"

    def valid(path):
        if not path.is_file() or path.stat().st_size != size:
            return False
        digest = hashlib.sha256()
        with path.open("rb") as file:
            while chunk := file.read(1024 * 1024):
                digest.update(chunk)
        return digest.hexdigest() == expected

    if valid(archive):
        print(f"Using verified {label} archive", flush=True)
        return archive
    for attempt in range(3):
        if partial.is_file() and partial.stat().st_size >= size:
            if valid(partial):
                partial.replace(archive)
                return archive
            partial.unlink()
        offset = partial.stat().st_size if partial.is_file() else 0
        request = urllib.request.Request(url, headers={"Range": f"bytes={offset}-"} if offset else {})
        print("UNITY_INSTALL_STAGE 2", flush=True)
        print(f"{label} model download attempt {attempt + 1}/3: {url}", flush=True)
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                status = response.status
                if status == 206:
                    match = re.fullmatch(r"bytes (\d+)-(\d+)/(\d+)", response.headers.get("Content-Range", ""))
                    if not match or int(match[1]) != offset or int(match[3]) != size or int(match[2]) != size - 1:
                        raise ValueError(f"Invalid {label} resume response")
                elif status == 200:
                    offset = 0  # Server ignored Range: restart, never append a full response.
                else:
                    raise ValueError(f"Unexpected {label} download status: {status}")
                downloaded = offset
                print(f"Progress {downloaded} of {size}", flush=True)
                with partial.open("ab" if offset else "wb") as file:
                    while chunk := response.read(1024 * 1024):
                        downloaded += len(chunk)
                        if downloaded > size:
                            raise ValueError(f"{label} archive exceeds the expected size")
                        file.write(chunk)
                        print(f"Progress {downloaded} of {size}", flush=True)
                if downloaded != size:
                    raise ConnectionError(f"Incomplete {label} model download")
            print("UNITY_INSTALL_STAGE 3", flush=True)
            if not valid(partial):
                partial.unlink(missing_ok=True)
                raise ValueError(f"{label} archive failed SHA-256 verification")
            partial.replace(archive)
            return archive
        except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as error:
            print(f"{label} download interrupted; partial data retained: {error}", flush=True)
            if attempt == 2:
                raise RuntimeError(f"{label} model download timed out or failed: {url}. Retry installation to resume. {error}") from error
            time.sleep(attempt + 1)
    raise RuntimeError(f"{label} download failed")

