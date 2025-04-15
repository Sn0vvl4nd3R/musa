from flask import Flask, request, jsonify
from flask_cors import CORS
from backend.app.models.user import UserBuilder, User
from .models.track import Track
from .models.playlist import PlaylistBuilder
from typing import Any, Dict
import uuid

app = Flask(__name__)
CORS(app)

users = {}
playlists = {}
tokens = {}

user_id_counter = 1
playlist_id_counter = 1

@app.route("/", methods=["GET"])
def index():
    return "API musa работает", 200

@app.route("/register", methods=["POST"])
def register():
    global user_id_counter
    data: Dict[str, Any] = request.get_json(silent=True)
    if data is None:
        return jsonify({"error": "Invalid or missing JSON"}), 400
    try:
        builder = UserBuilder()\
            .set_first_name(data.get("first_name"))\
            .set_last_name(data.get("last_name"))\
            .set_email(data.get("email"))\
            .set_password(data.get("password"))
        if "address" in data:
            builder.set_address(data["address"])
        if "passport" in data:
            builder.set_passport(data["passport"])
        user = builder.build()
        user.id = user_id_counter
        users[user_id_counter] = user
        user_id_counter += 1
        return jsonify({
            "id": user.id,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "email": user.email
        }), 201
    except Exception as e:
        return jsonify({"error": str(e)}), 400

@app.route("/login", methods=["POST"])
def login():
    data: Dict[str, Any] = request.get_json(silent=True)
    if data is None:
        return jsonify({"error": "Invalid or missing JSON"}), 400
    email = data.get("email")
    password = data.get("password")
    if not email or not password:
        return jsonify({"error": "Email and password required"}), 400
    user = None
    for uid, u in users.items():
        if u.email == email:
            user = u
            break
    if user is None:
        return jsonify({"error": "User not found"}), 404
    if not user.check_password(password):
        return jsonify({"error": "Invalid password"}), 401
    token = str(uuid.uuid4())
    tokens[token] = user.id
    return jsonify({"token": token, "user_id": user.id}), 200

def get_user_from_token() -> User:
    auth_header = request.headers.get("Authorization", "")
    if auth_header.startswith("Bearer "):
        token = auth_header.split(" ")[1]
        user_id = tokens.get(token)
        if user_id:
            return users.get(user_id)
    return None

@app.route("/playlist", methods=["GET", "POST"])
def playlist_handler():
    global playlist_id_counter
    user = get_user_from_token()
    if user is None:
        return jsonify({"error": "Unauthorized"}), 401

    if request.method == "POST":
        data: Dict[str, Any] = request.get_json(silent=True)
        if data is None:
            return jsonify({"error": "Invalid or missing JSON"}), 400
        try:
            builder = PlaylistBuilder().set_name(data.get("name", "Без названия")).set_owner(user.id)
            tracks_data = data.get("tracks", [])
            for t in tracks_data:
                track = Track(title=t.get("title"), artist=t.get("artist"), duration=t.get("duration"))
                builder.add_track(track)
            playlist = builder.build()
            playlist.id = playlist_id_counter
            playlists[playlist_id_counter] = playlist
            playlist_id_counter += 1
            return jsonify({
                "id": playlist.id,
                "name": playlist.name,
                "owner_id": playlist.owner_id,
                "tracks": [t.title for t in playlist.tracks]
            }), 201
        except Exception as e:
            return jsonify({"error": str(e)}), 400
    else:
        result = []
        for pid, playlist in playlists.items():
            if playlist.owner_id == user.id:
                result.append({
                    "id": pid,
                    "name": playlist.name,
                    "tracks": [t.title for t in playlist.tracks]
                })
        return jsonify(result), 200

if __name__ == '__main__':
    app.run(debug=True)
